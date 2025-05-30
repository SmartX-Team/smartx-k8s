use std::collections::VecDeque;

use crate::proc::ProcessID;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Interrupt {
    Full,
    Halt,
    Panic,
    Term,
}

pub struct Scheduler {
    rq: VecDeque<ProcessID>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            rq: Default::default(),
        }
    }
}

impl Scheduler {
    pub(crate) fn request(&mut self, pid: ProcessID) {
        self.rq.push_back(pid)
    }

    pub(crate) fn request_ahead(&mut self, pid: ProcessID) {
        self.rq.push_front(pid)
    }

    pub(crate) fn next(&mut self) -> ProcessID {
        self.rq.pop_front().expect("empty rq")
    }
}
