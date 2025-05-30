mod handler;
mod pipe;
mod proc;
mod prog;
mod sched;

use std::{
    ffi::CString,
    fs::File,
    io,
    os::fd::{FromRawFd, RawFd},
};

use anyhow::{Result, bail};
use dark_lake_api::kernel::VirtualMachineInstance;
use io_uring::{
    CompletionQueue, IoUring, SubmissionQueue, Submitter,
    squeue::{Entry, Flags, PushError},
    types,
};
#[cfg(feature = "tracing")]
use tracing::info;

use crate::{
    handler::{Context, Handler as _, Phase},
    pipe::PipeID,
    proc::{ControlGroups, ProcessID},
    prog::{Program, ProgramState},
    sched::{Interrupt, Scheduler},
};

fn open_file(path: &str, oflag: ::libc::c_int, mode: ::libc::c_uint) -> io::Result<RawFd> {
    let c_path = CString::new(path)?;
    let fd = unsafe { ::libc::open(c_path.as_ptr(), oflag, mode) };
    if fd == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

struct Handler<'a> {
    capacity: usize,
    phase: Phase,
    sq: SubmissionQueue<'a>,
    submitter: Submitter<'a>,
}

impl crate::handler::Handler for Handler<'_> {
    #[inline]
    fn get_phase(&self) -> Phase {
        self.phase
    }

    #[inline]
    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    #[inline]
    fn poll_closure(&mut self, ctx: &mut Context, entry: Entry) -> Result<(), Interrupt> {
        if self.capacity == 0 {
            return Err(Interrupt::Full);
        }
        self.capacity -= 1;

        let Context { pid, pipe } = ctx;
        let data = {
            let pid = u32::from(*pid) as u64;
            let pipe = u32::from(*pipe) as u64;
            pid << (size_of::<u32>() * 8) | pipe
        };

        match unsafe { self.sq.push(&entry.user_data(data)) } {
            Ok(()) => Ok(()),
            Err(PushError { .. }) => Err(Interrupt::Full),
        }
    }

    #[inline]
    fn exit_closure(&mut self, _ctx: Context) -> Result<(), Interrupt> {
        Ok(())
    }
}

impl Handler<'_> {
    #[inline]
    fn commit(&mut self, cq: &mut CompletionQueue<'_>, want: usize) -> io::Result<()> {
        self.sq.sync();
        self.submitter.submit_and_wait(want)?;
        cq.sync();
        self.capacity += cq.len();
        Ok(())
    }
}

#[derive(Default)]
pub struct Kernel {
    ns: ControlGroups,
    sched: Scheduler,
}

impl ::dark_lake_api::kernel::Kernel for Kernel {
    fn wait(&mut self, vmi: VirtualMachineInstance) -> Result<()> {
        let Self { ns, sched } = self;

        // Define kernel processes
        {
            let prog = Program::new(ProgramState::Signal(self::prog::Signal::new()?)).on_kernel();
            let id = ns.spawn(prog);
            sched.request(id)
        }

        // Define programs

        let mode = 0o644;
        let fd_in_raw = open_file(
            "/tmp/bigfile",
            ::libc::O_RDONLY | ::libc::O_NONBLOCK | ::libc::O_DIRECT,
            mode,
        )?;
        let fd_out_raw = open_file(
            "/tmp/bigfile2",
            ::libc::O_WRONLY
                | ::libc::O_NONBLOCK
                | ::libc::O_DIRECT
                | ::libc::O_CREAT
                | ::libc::O_TRUNC,
            mode,
        )?;

        let block_size = 1 * 1024 * 1024; // 1MB
        // for fd in [fd_in_raw, fd_out_raw] {
        //     let ret = unsafe { ::libc::fcntl(fd, ::libc::F_SETPIPE_SZ, block_size) };
        //     if ret == -1 {
        //         let error = io::Error::last_os_error();
        //         bail!("Failed to create a pipe: {error}");
        //     };
        // }

        let src = unsafe { File::from_raw_fd(fd_in_raw) };
        let sink = unsafe { File::from_raw_fd(fd_out_raw) };

        let entries = 64;
        let mut ring: IoUring = IoUring::builder()
            .dontfork()
            .setup_single_issuer()
            .build(entries)?;

        let fd_in = types::Fd(fd_in_raw);
        let fd_out = types::Fd(fd_out_raw);

        let total_len = src.metadata()?.len() as _;

        let mut pipe2: [i32; 2] = [0; 2];
        {
            let ret = unsafe {
                ::libc::pipe2(
                    pipe2.as_mut_ptr(),
                    ::libc::O_CLOEXEC | ::libc::O_NONBLOCK | ::libc::O_DIRECT,
                )
            };
            if ret == -1 {
                let error = io::Error::last_os_error();
                bail!("Failed to create a pipe: {error}");
            }
        }
        {
            let fd = pipe2[0];
            let ret = unsafe { ::libc::fcntl(fd, ::libc::F_SETPIPE_SZ, block_size) };
            if ret == -1 {
                let error = io::Error::last_os_error();
                bail!("Failed to create a pipe: {error}");
            };
        }

        let [pipe_rd, pipe_wr] = pipe2;
        let pipe_rd = types::Fd(pipe_rd);
        let pipe_wr = types::Fd(pipe_wr);

        // Register programs
        {
            let proc_rd = ns.spawn(Program::new(ProgramState::Splice(self::prog::Splice {
                fd_in,
                fd_out: pipe_wr,
                off_in: 0,
                off_out: -1,
                buf: block_size,
                remaining: total_len,
                flags: Flags::IO_LINK,
            })));
            let proc_wr = ns.spawn(
                Program::new(ProgramState::Splice(self::prog::Splice {
                    fd_in: pipe_rd,
                    fd_out,
                    off_in: -1,
                    off_out: 0,
                    buf: block_size,
                    remaining: total_len,
                    flags: Flags::IO_LINK,
                }))
                .with_src(vec![proc_rd]),
            );

            sched.request(proc_rd);
            sched.request(proc_wr);
        }

        // Boot
        let (submitter, sq, mut cq) = ring.split();
        let mut handler = Handler {
            capacity: entries as _,
            phase: Default::default(),
            sq,
            submitter,
        };
        loop {
            let pid = sched.next();
            let proc = ns.proc(pid);
            match proc.poll(&mut handler) {
                Ok(()) => sched.request(pid),
                Err(Interrupt::Full) => {
                    sched.request_ahead(pid);
                    handler.commit(&mut cq, 1)?;
                    continue;
                }
                Err(Interrupt::Halt) => {
                    {
                        #[cfg(feature = "tracing")]
                        info!("Terminating...");
                        handler.set_phase(Phase::Terminating);
                    }
                    ns.terminate(pid);
                    if ns.completed() {
                        ns.exit();
                        break Ok(());
                    }
                }
                Err(Interrupt::Panic) => panic!("kernel panic"),
                Err(Interrupt::Term) => {
                    ns.terminate(pid);
                    if ns.completed() {
                        ns.exit();
                        break Ok(());
                    }
                }
            }
            for entry in &mut cq {
                let data = entry.user_data();
                let pid = ProcessID::new_unchecked((data >> (size_of::<u32>() * 8)) as u32);
                let pipe = PipeID::new_unchecked((data & (1 << (size_of::<u32>() * 8) - 1)) as u32);
                sched.request(pid);
                dbg!(pid, pipe);
            }
        }
    }
}
