use std::{collections::VecDeque, io};

use crate::proc::ProcessID;

pub type Result<T = State, E = Interrupt> = ::core::result::Result<T, E>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Retry,
    Sleep,
    Terminate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Interrupt {
    EmptyPipe,
    Full,
    Halt,
    System(SystemInterrupt),
}

impl From<io::Error> for Interrupt {
    #[inline]
    fn from(error: io::Error) -> Self {
        Self::System(error.into())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SystemInterrupt {
    Completed,
    Panic,
}

impl From<io::Error> for SystemInterrupt {
    fn from(error: io::Error) -> Self {
        #[cfg(feature = "tracing")]
        ::tracing::error!("{error}");
        let _ = error;
        Self::Panic
    }
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

    pub(crate) fn request_batch(&mut self, pids: &[ProcessID]) {
        for &pid in pids {
            self.rq.push_back(pid);
        }
    }

    pub(crate) fn next(&mut self) -> Option<ProcessID> {
        self.rq.pop_front()
    }
}
