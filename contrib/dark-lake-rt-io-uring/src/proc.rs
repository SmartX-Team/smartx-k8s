use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt, io,
};

#[cfg(feature = "tracing")]
use tracing::info;

use crate::{
    handler::{Handler, HandlerExt},
    pipe::{Pipe, Pipe2, Pipe2Pool, PipePool},
    prog::Program,
    sched::{Interrupt, Result, State, SystemInterrupt},
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Init,
    Running,
    Terminating,
}

impl Phase {
    pub(crate) fn set_running(&mut self) {
        if matches!(self, Self::Init) {
            *self = Self::Running;
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct ProcessID(u32);

impl fmt::Debug for ProcessID {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for ProcessID {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<ProcessID> for u32 {
    #[inline]
    fn from(value: ProcessID) -> Self {
        value.0
    }
}

impl ProcessID {
    pub(crate) const INIT: Self = Self(1);
    pub(crate) const MAX: Self = Self(u32::MAX);

    #[inline]
    pub(crate) const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }

    pub(crate) fn next(&mut self) -> Self {
        let id = *self;
        if *self == Self::MAX {
            self.0 = Self::INIT.0 + 1;
        } else {
            self.0 += 1;
        }
        id
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Context {
    id: ProcessID,
    phase: Phase,
    pub(crate) count: usize,
}

impl Context {
    #[inline]
    pub(crate) const fn pid(&self) -> ProcessID {
        self.id
    }

    #[inline]
    pub(crate) const fn phase(&self) -> Phase {
        self.phase
    }

    #[inline]
    pub(crate) const fn count(&self) -> usize {
        self.count
    }
}

#[derive(Debug)]
pub(crate) struct Process {
    ctx: Context,
    prog: Program,
    next: Vec<ProcessID>,
    sink: Vec<ProcessID>,
}

impl Process {
    #[inline]
    pub(crate) const fn pid(&self) -> ProcessID {
        self.ctx.pid()
    }

    #[inline]
    pub(crate) const fn phase(&self) -> Phase {
        self.ctx.phase()
    }

    #[inline]
    pub(crate) const fn kernel(&self) -> bool {
        self.prog.kernel()
    }

    #[inline]
    pub(crate) fn poll<H>(&mut self, handler: &mut H) -> Result
    where
        H: Handler,
    {
        handler.poll_within(self.ctx, |closure| self.prog.ctx.poll(closure))
    }
}

#[derive(Debug)]
pub(crate) enum Next<'a> {
    Forget,
    Reschedule,
    Retry,
    Later(Cow<'a, [ProcessID]>),
}

pub(crate) struct ControlGroups {
    kernel: HashSet<ProcessID>,
    pipe: PipePool,
    pipe2: Pipe2Pool,
    proc: HashMap<ProcessID, Process>,
    seed: ProcessID,
    terminating: bool,
}

impl Default for ControlGroups {
    fn default() -> Self {
        Self {
            kernel: Default::default(),
            pipe: PipePool::default(),
            pipe2: Pipe2Pool::new(1 << 20),
            proc: Default::default(),
            seed: ProcessID::INIT,
            terminating: false,
        }
    }
}

impl ControlGroups {
    fn completed(&self) -> bool {
        self.kernel.len() <= self.proc.len()
            && self.proc.keys().all(|pid| self.kernel.contains(pid))
    }

    fn exit(&mut self) {
        self.kernel.clear();
        self.proc.clear();
        self.seed = ProcessID::INIT;
        self.terminating = false;
    }

    #[inline]
    pub(crate) fn alloc_file(
        &mut self,
        path: &str,
        oflag: ::libc::c_int,
        mode: ::libc::c_uint,
    ) -> io::Result<Pipe> {
        self.pipe.register_file(path, oflag, mode)
    }

    #[inline]
    pub(crate) fn alloc_pipe(&mut self) -> io::Result<Pipe2> {
        self.pipe2.dequeue()
    }

    #[inline]
    pub(crate) fn dealloc_pipe(&mut self, pipe: Pipe) {
        self.pipe2.enqueue(pipe)
    }

    pub(crate) fn spawn(&mut self, prog: Program) -> ProcessID {
        let pid = self.seed.next();
        if prog.kernel() {
            self.kernel.insert(pid);
        }

        let proc = Process {
            ctx: Context {
                id: pid,
                phase: Default::default(),
                count: 0,
            },
            prog,
            next: Default::default(),
            sink: Default::default(),
        };
        self.proc.insert(pid, proc);
        pid
    }

    #[inline]
    pub(crate) fn poll<H>(
        &mut self,
        pid: ProcessID,
        handler: &mut H,
    ) -> Result<Next<'_>, SystemInterrupt>
    where
        H: Handler,
    {
        let proc = match self.proc.get_mut(&pid) {
            Some(proc) => proc,
            None => panic!("missing process: {pid}"),
        };
        if self.terminating {
            proc.ctx.phase = Phase::Terminating;
        }

        let result = proc.poll(handler);
        match result {
            Ok(State::Retry) => Ok(Next::Retry),
            Ok(State::Sleep) => {
                proc.ctx.phase.set_running();
                Ok(Next::Forget)
            }
            Ok(State::Terminate) => self.terminate(pid),
            Err(Interrupt::EmptyPipe) => Ok(Next::Retry),
            Err(Interrupt::Full) => Ok(Next::Reschedule),
            Err(Interrupt::Halt) => {
                {
                    #[cfg(feature = "tracing")]
                    info!("Terminating...");
                    self.terminating = true;
                    proc.ctx.phase = Phase::Terminating;
                }
                self.terminate(pid)
            }
            Err(Interrupt::System(interrupt)) => Err(interrupt),
        }
    }

    fn terminate(&mut self, pid: ProcessID) -> Result<Next<'_>, SystemInterrupt> {
        self.kernel.remove(&pid);
        let proc = self.proc.remove(&pid).expect("missing process");

        // FIXME: return pipe & pipe2
        if !proc.next.is_empty() {
            Ok(Next::Later(Cow::Owned(proc.next)))
        } else if self.completed() {
            Err(SystemInterrupt::Completed)
        } else {
            Ok(Next::Forget)
        }
    }
}
