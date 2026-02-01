mod handler;
mod pipe;
mod proc;
mod prog;
mod sched;

use std::io;

use anyhow::Result;
use bitflags::bitflags;
use dark_lake_api::kernel::CompiledScript;
use io_uring::{CompletionQueue, IoUring, SubmissionQueue, Submitter, squeue::Entry};

use crate::{
    proc::{Context, ControlGroups, Next, ProcessID},
    prog::{Program, ProgramState},
    sched::{Interrupt, Result as PollResult, Scheduler, SystemInterrupt},
};

bitflags! {
    #[derive(Copy, Clone, Debug, Default)]
    struct Flags: u32 {
        const EOS = 0b_0000_0000_0000_0000_0000_0000_0000_0001;
    }
}

struct Handler<'a> {
    buf: Vec<Entry>,
    capacity: usize,
    sq: SubmissionQueue<'a>,
    submitter: Submitter<'a>,
}

impl crate::handler::Handler for Handler<'_> {
    #[inline]
    fn poll_closure(&mut self, ctx: &mut Context, entry: Entry) -> PollResult<()> {
        if self.is_full() {
            return Err(Interrupt::Full);
        }
        self.capacity -= 1;

        let pid = (u32::from(ctx.pid()) as u64) << (u32::BITS as usize);
        let flags = Flags::empty();
        let data = pid | flags.bits() as u64;

        self.buf.push(entry.user_data(data));
        Ok(())
    }

    #[inline]
    fn exit_closure(&mut self, ctx: Context) {
        if ctx.count() > 0 {
            let entry = self.buf.pop().unwrap();
            let data = entry.get_user_data() | Flags::EOS.bits() as u64;
            self.buf.push(entry.user_data(data));
        }
    }
}

impl Handler<'_> {
    const fn is_full(&self) -> bool {
        self.capacity == 0
    }

    #[inline]
    fn cancel(&mut self, cq: &mut CompletionQueue<'_>, want: usize) -> io::Result<()> {
        self.buf.clear();

        if want > 0 {
            self.submitter.submit_and_wait(want)?;
        }
        cq.sync();
        self.capacity += cq.len();
        Ok(())
    }

    #[inline]
    fn commit(&mut self, cq: &mut CompletionQueue<'_>, want: usize) -> io::Result<()> {
        unsafe { self.sq.push_multiple(&self.buf).unwrap_unchecked() };
        self.buf.clear();
        self.sq.sync();

        self.submitter.submit_and_wait(want)?;
        cq.sync();
        self.capacity += cq.len();
        Ok(())
    }
}

#[derive(Default)]
pub struct Kernel {
    groups: ControlGroups,
    sched: Scheduler,
}

impl ::dark_lake_api::kernel::Kernel for Kernel {
    fn wait(&mut self, script: CompiledScript) -> Result<()> {
        let Self { groups, sched } = self;

        // Create a ring
        let entries = 4;
        let mut ring: IoUring = IoUring::builder()
            .dontfork()
            .setup_single_issuer()
            .build(entries)?;

        // Define kernel processes
        {
            let prog = Program::new(ProgramState::Signal(self::prog::Signal::new()?)).on_kernel();
            let id = groups.spawn(prog);
            sched.request(id)
        }

        // Register programs
        // FIXME: parse from compiled script
        // FIXME: CompiledScript == Vec<Program>
        {
            let mode = 0o644;

            let fd_in =
                groups.alloc_file("/tmp/bigfile", ::libc::O_RDONLY | ::libc::O_NONBLOCK, mode)?;
            let fd_out = groups.alloc_file(
                "/tmp/bigfile2",
                ::libc::O_WRONLY
                    | ::libc::O_NONBLOCK
                    | ::libc::O_DIRECT
                    | ::libc::O_CREAT
                    | ::libc::O_TRUNC,
                mode,
            )?;

            {
                let self::pipe::Pipe2 { tx, rx } = groups.alloc_pipe()?;
                let proc = groups.spawn(Program::new(ProgramState::Serial(self::prog::Serial(
                    vec![
                        ProgramState::Splice(self::prog::Splice {
                            rx: fd_in,
                            tx,
                            remaining: -1,
                        }),
                        ProgramState::Splice(self::prog::Splice {
                            rx,
                            tx: fd_out,
                            remaining: -1,
                        }),
                    ],
                ))));
                sched.request(proc);
            }
        }

        // Boot
        let (submitter, sq, mut cq) = ring.split();
        let mut handler = Handler {
            buf: Default::default(),
            capacity: entries as _,
            sq,
            submitter,
        };
        loop {
            match sched.next() {
                Some(pid) => {
                    let result = groups.poll(pid, &mut handler);
                    match result {
                        Ok(Next::Forget) => (),
                        Ok(Next::Later(pids)) => {
                            handler.commit(&mut cq, 0)?;
                            sched.request_batch(&pids)
                        }
                        Ok(Next::Reschedule) => {
                            handler.cancel(&mut cq, 1)?;
                            sched.request_ahead(pid)
                        }
                        Ok(Next::Retry) => {
                            handler.cancel(&mut cq, 0)?;
                            sched.request(pid)
                        }
                        Err(SystemInterrupt::Completed) => break Ok(()),
                        Err(SystemInterrupt::Panic) => panic!("kernel panic"),
                    }
                }
                None => handler.commit(&mut cq, 1)?,
            }
            for entry in &mut cq {
                let data = entry.user_data();
                let pid = ProcessID::new_unchecked((data >> (u32::BITS as usize)) as _);
                let flags = Flags::from_bits_truncate(data as _);

                #[cfg(feature = "tracing")]
                {
                    let result = entry.result();
                    if result < 0 {
                        ::tracing::error!(
                            "Error while processing PID {pid}: {}",
                            io::Error::from_raw_os_error(-result),
                        );
                    }
                }

                if flags.contains(Flags::EOS) {
                    sched.request(pid);
                }
            }
        }
    }
}
