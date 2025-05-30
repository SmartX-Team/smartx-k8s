use std::{collections::VecDeque, fmt, io};

use io_uring::types::Fd;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct PipeID(u32);

impl fmt::Debug for PipeID {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for PipeID {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for PipeID {
    #[inline]
    fn default() -> Self {
        Self(u32::MAX)
    }
}

impl From<PipeID> for u32 {
    #[inline]
    fn from(value: PipeID) -> Self {
        value.0
    }
}

impl PipeID {
    #[inline]
    pub(crate) const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Pipe {
    tx: Fd,
    rx: Fd,
}

impl Pipe {
    fn new(size: i32) -> io::Result<Self> {
        let mut pipe2 = [0; 2];
        {
            let ret = unsafe {
                ::libc::pipe2(
                    pipe2.as_mut_ptr(),
                    ::libc::O_CLOEXEC | ::libc::O_NONBLOCK | ::libc::O_DIRECT,
                )
            };
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        if size > 0 {
            let fd = pipe2[0];
            let ret = unsafe { ::libc::fcntl(fd, ::libc::F_SETPIPE_SZ, size) };
            if ret == -1 {
                return Err(io::Error::last_os_error());
            };
        }
        let [tx, rx] = pipe2;
        Ok(Self {
            tx: Fd(tx),
            rx: Fd(rx),
        })
    }
}

pub(crate) struct PipePool {
    pool: Vec<Pipe>,
    queue: VecDeque<PipeID>,
    size: PipeID,
}

impl PipePool {
    pub(crate) fn new(num_pipes: u32, pipe_size: i32) -> io::Result<Self> {
        Ok(Self {
            pool: (0..num_pipes)
                .map(|_| Pipe::new(pipe_size))
                .collect::<io::Result<_>>()?,
            queue: (0..num_pipes).map(PipeID).collect(),
            size: PipeID(num_pipes),
        })
    }
}
