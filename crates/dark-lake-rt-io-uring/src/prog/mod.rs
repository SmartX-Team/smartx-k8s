use io_uring::opcode;

use crate::{
    handler::{Closure, Handler, Phase},
    proc::ProcessID,
    sched::Interrupt,
};

mod signal;
mod splice;

pub use self::{signal::Signal, splice::Splice};

#[derive(Debug, Default)]
pub enum ProgramState {
    #[default]
    Nop,
    Signal(Signal),
    Splice(Splice),
}

impl ProgramState {
    pub(crate) fn poll<H>(&mut self, closure: &mut Closure<'_, H>) -> Result<(), Interrupt>
    where
        H: Handler,
    {
        let phase = closure.phase();

        match self {
            ProgramState::Nop => Err(Interrupt::Term),
            ProgramState::Signal(Signal { fd, buf }) => match phase {
                Phase::Init => {
                    let fd = *fd;
                    let len = size_of_val(buf) as _;
                    let buf = (&raw mut *buf) as _;
                    let entry = opcode::Read::new(fd, buf, len).build();
                    closure.poll(entry)?;
                    Ok(())
                }
                Phase::Running | Phase::Terminating => {
                    let signal = unsafe { buf.assume_init_ref() };
                    match signal.ssi_signo as _ {
                        ::libc::SIGINT | ::libc::SIGTERM => Err(Interrupt::Halt),
                        signo => unreachable!("Received unhandled signal: {signo}"),
                    }
                }
            },
            ProgramState::Splice(ctx) => {
                if matches!(phase, Phase::Init) {
                    return Ok(());
                }
                if matches!(phase, Phase::Terminating) || ctx.remaining == 0 {
                    return Err(Interrupt::Term);
                }

                let chunk = if ctx.remaining != -1 {
                    let chunk = (ctx.buf as i64).min(ctx.remaining);
                    ctx.remaining -= chunk;
                    chunk
                } else {
                    ctx.buf as _
                };

                if ctx.off_in != -1 {
                    ctx.off_in += chunk;
                }
                if ctx.off_out != -1 {
                    ctx.off_out += chunk;
                }

                let Splice {
                    fd_in,
                    fd_out,
                    off_in,
                    off_out,
                    flags: squeue_flags,
                    ..
                } = *ctx;

                let mut flags = ::libc::SPLICE_F_NONBLOCK | ::libc::SPLICE_F_MOVE;
                if ctx.remaining != 0 {
                    flags |= ::libc::SPLICE_F_MORE;
                }

                let len = chunk as _;
                let entry = opcode::Splice::new(fd_in, off_in, fd_out, off_out, len)
                    .flags(flags)
                    .build()
                    .flags(squeue_flags);
                closure.poll(entry)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Program {
    pub(crate) ctx: ProgramState,
    depends: Vec<ProcessID>,
    kernel: bool,
    src: Vec<ProcessID>,
}

impl Program {
    pub fn new(ctx: ProgramState) -> Self {
        Self {
            ctx,
            depends: Default::default(),
            kernel: false,
            src: Default::default(),
        }
    }

    #[inline]
    pub fn with_src(mut self, src: Vec<ProcessID>) -> Self {
        self.src = src;
        self
    }

    #[inline]
    pub fn on_kernel(mut self) -> Self {
        self.kernel = true;
        self
    }

    #[inline]
    pub const fn kernel(&self) -> bool {
        self.kernel
    }

    #[inline]
    pub(crate) const fn depends(&self) -> &[ProcessID] {
        self.depends.as_slice()
    }

    #[inline]
    pub const fn src(&self) -> &[ProcessID] {
        self.src.as_slice()
    }
}
