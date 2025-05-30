use io_uring::{opcode, types};

use crate::{
    handler::{Closure, Handler},
    proc::Phase,
    sched::{Interrupt, Result, State},
};

mod serial;
mod signal;
mod splice;

pub use self::{serial::Serial, signal::Signal, splice::Splice};

#[derive(Debug, Default)]
pub enum ProgramState {
    #[default]
    Nop,
    Serial(Serial),
    Signal(Signal),
    Splice(Splice),
}

impl ProgramState {
    pub(crate) fn poll<H>(&mut self, closure: &mut Closure<'_, H>) -> Result
    where
        H: Handler,
    {
        let phase = closure.phase();

        match self {
            Self::Nop => Ok(State::Terminate),
            Self::Serial(Serial(children)) => {
                for child in children {
                    match child.poll(closure)? {
                        State::Retry => return Ok(State::Retry),
                        State::Sleep => continue,
                        State::Terminate => return Ok(State::Terminate),
                    }
                }
                Ok(State::Sleep)
            }
            Self::Signal(Signal { fd, buf }) => match phase {
                Phase::Init => {
                    let fd = *fd;
                    let len = size_of_val(buf) as _;
                    let buf = (&raw mut *buf) as _;
                    let entry = opcode::Read::new(fd, buf, len).build();
                    closure.poll(entry)
                }
                Phase::Running | Phase::Terminating => {
                    let signal = unsafe { buf.assume_init_ref() };
                    match signal.ssi_signo as _ {
                        ::libc::SIGINT | ::libc::SIGTERM => Err(Interrupt::Halt),
                        signo => unreachable!("Received unhandled signal: {signo}"),
                    }
                }
            },
            Self::Splice(Splice { rx, tx, remaining }) => {
                if matches!(phase, Phase::Terminating) || *remaining == 0 {
                    return Ok(State::Terminate);
                }

                let buf_size = match (rx.buf_size(), tx.buf_size()) {
                    (-1, -1) => -1,
                    (-1, size) | (size, -1) => size,
                    (a, b) => a.min(b),
                };

                if *remaining == -1 && rx.offset != -1 && rx.len != -1 {
                    *remaining = rx.len - rx.offset;
                }
                let chunk = if *remaining != -1 {
                    let chunk = (buf_size as i64).min(*remaining);
                    *remaining -= chunk;
                    chunk
                } else {
                    buf_size as _
                };

                let off_in = rx.offset;
                if off_in != -1 {
                    rx.offset += chunk;
                }
                let fd_in = types::Fd(rx.fd());

                let off_out = tx.offset;
                if off_out != -1 {
                    tx.offset += chunk;
                }
                let fd_out = types::Fd(tx.fd());

                let mut flags = ::libc::SPLICE_F_NONBLOCK | ::libc::SPLICE_F_MOVE;
                if *remaining > 0 {
                    flags |= ::libc::SPLICE_F_MORE;
                }

                let len = chunk as _;
                let entry = opcode::Splice::new(fd_in, off_in, fd_out, off_out, len)
                    .flags(flags)
                    .build();
                closure.poll(entry)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Program {
    pub(crate) ctx: ProgramState,
    kernel: bool,
}

impl Program {
    pub fn new(ctx: ProgramState) -> Self {
        Self { ctx, kernel: false }
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
}
