use io_uring::squeue::Entry;

use crate::{
    proc::{Context, Phase},
    sched::{Result, State},
};

pub struct Closure<'h, H> {
    pub(crate) ctx: Context,
    handler: &'h mut H,
}

impl<H> Closure<'_, H>
where
    H: Handler,
{
    #[inline]
    pub(crate) const fn phase(&self) -> Phase {
        self.ctx.phase()
    }

    #[inline]
    pub(crate) fn poll(&mut self, entry: Entry) -> Result {
        self.handler.poll_closure(&mut self.ctx, entry)?;
        self.ctx.count += 1;
        Ok(State::Sleep)
    }
}

pub trait Handler {
    fn poll_closure(&mut self, ctx: &mut Context, entry: Entry) -> Result<()>;

    fn exit_closure(&mut self, ctx: Context);
}

pub trait HandlerExt
where
    Self: Handler,
{
    #[inline]
    fn poll_within<F>(&mut self, ctx: Context, f: F) -> Result
    where
        Self: Sized,
        F: FnOnce(&mut Closure<Self>) -> Result,
    {
        let mut closure = Closure { ctx, handler: self };
        let result = f(&mut closure);

        let ctx = closure.ctx;
        self.exit_closure(ctx);
        result
    }
}

impl<T> HandlerExt for T where Self: Handler {}
