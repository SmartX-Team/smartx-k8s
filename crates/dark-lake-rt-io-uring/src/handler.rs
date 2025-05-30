use io_uring::squeue::Entry;

use crate::{pipe::PipeID, proc::ProcessID, sched::Interrupt};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Init,
    Running,
    Terminating,
}

pub struct Context {
    pub(crate) pid: ProcessID,
    pub(crate) pipe: PipeID,
}

pub trait Handler {
    fn get_phase(&self) -> Phase;

    fn set_phase(&mut self, phase: Phase);

    fn poll_closure(&mut self, ctx: &mut Context, entry: Entry) -> Result<(), Interrupt>;

    fn exit_closure(&mut self, ctx: Context) -> Result<(), Interrupt>;
}

pub trait HandlerExt
where
    Self: Handler,
{
    #[inline]
    fn poll_within<F>(&mut self, pid: ProcessID, f: F) -> Result<(), Interrupt>
    where
        Self: Sized,
        F: FnOnce(&mut Closure<Self>) -> Result<(), Interrupt>,
    {
        let phase = self.get_phase();
        let mut closure = Closure {
            ctx: Context {
                pid,
                pipe: Default::default(),
            },
            handler: self,
            phase,
        };
        f(&mut closure)?;

        let ctx = closure.ctx;
        self.exit_closure(ctx)
    }
}

impl<T> HandlerExt for T where Self: Handler {}

pub struct Closure<'a, H> {
    ctx: Context,
    handler: &'a mut H,
    phase: Phase,
}

impl<H> Closure<'_, H>
where
    H: Handler,
{
    #[inline]
    pub(crate) const fn phase(&self) -> Phase {
        self.phase
    }

    #[inline]
    pub(crate) fn poll(&mut self, entry: Entry) -> Result<(), Interrupt> {
        self.handler.poll_closure(&mut self.ctx, entry)
    }
}
