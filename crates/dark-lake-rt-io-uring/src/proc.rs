use std::{collections::HashMap, fmt};

use crate::{
    handler::{Handler, HandlerExt},
    pipe::PipePool,
    prog::Program,
    sched::Interrupt,
};

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

#[derive(Debug)]
pub(crate) struct Process {
    id: ProcessID,
    prog: Program,
    pub(crate) next: Vec<ProcessID>,
    sink: Vec<ProcessID>,
}

impl Process {
    #[inline]
    pub(crate) const fn kernel(&self) -> bool {
        self.prog.kernel()
    }

    #[inline]
    pub(crate) fn poll<H>(&mut self, handler: &mut H) -> Result<(), Interrupt>
    where
        H: Handler,
    {
        handler.poll_within(self.id, |closure| self.prog.ctx.poll(closure))
    }
}

pub(crate) struct ControlGroups {
    kernel: Vec<ProcessID>,
    pipes: PipePool,
    proc: HashMap<ProcessID, Process>,
    seed: ProcessID,
}

impl Default for ControlGroups {
    fn default() -> Self {
        Self {
            kernel: Default::default(),
            pipes: PipePool::new(4, 1 << 20).expect("failed to build pipe pool"),
            proc: Default::default(),
            seed: ProcessID::INIT,
        }
    }
}

impl ControlGroups {
    pub(crate) fn completed(&self) -> bool {
        self.kernel.len() == self.proc.len()
    }

    pub(crate) fn exit(&mut self) {
        self.kernel.clear();
        self.proc.clear();
        self.seed = ProcessID::INIT;
    }

    pub(crate) fn spawn(&mut self, prog: Program) -> ProcessID {
        let pid = self.seed.next();
        if prog.kernel() {
            self.kernel.push(pid);
        }
        for src in prog.depends() {
            let proc = self.proc.get_mut(src).expect("missing process");
            proc.next.push(pid);
        }
        for src in prog.src() {
            let proc = self.proc.get_mut(src).expect("missing process");
            proc.sink.push(pid);
        }

        let proc = Process {
            id: pid,
            prog,
            next: Default::default(),
            sink: Default::default(),
        };
        self.proc.insert(pid, proc);
        pid
    }

    #[inline]
    pub(crate) fn proc(&mut self, pid: ProcessID) -> &mut Process {
        match self.proc.get_mut(&pid) {
            Some(proc) => proc,
            None => panic!("missing process: {pid}"),
        }
    }

    pub(crate) fn terminate(&mut self, pid: ProcessID) -> Vec<ProcessID> {
        self.proc
            .remove(&pid)
            .map(|proc| proc.next)
            .unwrap_or_default()
    }
}
