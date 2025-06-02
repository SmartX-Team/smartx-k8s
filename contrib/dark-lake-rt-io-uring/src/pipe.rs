use std::{
    collections::{HashMap, VecDeque},
    ffi::CString,
    fmt,
    fs::File,
    io, ops,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
};

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
        Self(0)
    }
}

impl From<PipeID> for u32 {
    #[inline]
    fn from(value: PipeID) -> Self {
        value.0
    }
}

impl PipeID {
    const INIT: Self = Self(1);
    const MAX: Self = Self(u32::MAX);

    #[inline]
    pub(crate) const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }

    pub(crate) fn next(&mut self) -> Self {
        let id = *self;
        if *self == Self::MAX {
            self.0 = Self::INIT.0;
        } else {
            self.0 += 1;
        }
        id
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct PipeKind {
    tx: bool,
    rx: bool,
}

impl PipeKind {
    const FILE: Self = PipeKind { tx: true, rx: true };

    const TX: Self = PipeKind {
        tx: true,
        rx: false,
    };

    const RX: Self = PipeKind {
        tx: false,
        rx: true,
    };
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct PipeState {
    pub(crate) offset: i64,
    pub(crate) len: i64,
}

impl PipeState {
    const UNLIMITED: Self = Self {
        offset: -1,
        len: -1,
    };
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Pipe<FD = RawFd> {
    kind: PipeKind,
    id: PipeID,
    fd: FD,
    buf_size: i32,
    state: PipeState,
}

impl<FD> ops::Deref for Pipe<FD> {
    type Target = PipeState;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<FD> ops::DerefMut for Pipe<FD> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<FD> Pipe<FD>
where
    FD: AsRawFd,
{
    #[inline]
    fn as_ptr(&self) -> Pipe {
        Pipe {
            kind: self.kind,
            id: self.id,
            fd: self.fd.as_raw_fd(),
            buf_size: self.buf_size,
            state: self.state,
        }
    }

    #[inline]
    fn update(&mut self, other: Pipe) {
        self.state = other.state
    }
}

impl Pipe<File> {
    pub(crate) fn open_file(
        id: PipeID,
        path: &str,
        oflag: ::libc::c_int,
        mode: ::libc::c_uint,
    ) -> io::Result<Self> {
        let c_path = CString::new(path)?;
        let fd = unsafe { ::libc::open(c_path.as_ptr(), oflag, mode) };
        if fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            let file = unsafe { File::from_raw_fd(fd) };
            let len = if mode as i32 & ::libc::O_WRONLY == ::libc::O_WRONLY {
                0
            } else {
                file.metadata()?.len() as _
            };
            Ok(Self {
                kind: PipeKind::FILE,
                id,
                fd: file,
                buf_size: -1,
                state: PipeState { offset: 0, len },
            })
        }
    }
}

impl Pipe {
    #[inline]
    pub(crate) const fn fd(&self) -> RawFd {
        self.fd
    }

    #[inline]
    pub(crate) const fn buf_size(&self) -> i32 {
        self.buf_size
    }
}

#[derive(Debug)]
struct PipePin {
    data: Pipe<File>,
}

#[derive(Default)]
pub(crate) struct PipePool {
    pool: HashMap<PipeID, PipePin>,
    seed: PipeID,
}

impl PipePool {
    pub(crate) fn register_file(
        &mut self,
        path: &str,
        oflag: ::libc::c_int,
        mode: ::libc::c_uint,
    ) -> io::Result<Pipe> {
        let id = self.seed.next();
        let pipe = Pipe::open_file(id, path, oflag, mode)?;
        let ptr = pipe.as_ptr();
        let pin = PipePin { data: pipe };
        self.pool.insert(id, pin);
        Ok(ptr)
    }

    pub(crate) fn enqueue(&mut self, pipe: Pipe) {
        let id = pipe.id;
        let pin = self.pool.get_mut(&id).expect("missing pipe");
        pin.data.update(pipe)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Pipe2<FD = RawFd> {
    pub(crate) tx: Pipe<FD>,
    pub(crate) rx: Pipe<FD>,
}

impl Pipe2<OwnedFd> {
    fn new(id: PipeID, buf_size: i32) -> io::Result<Self> {
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
        if buf_size > 0 {
            let fd = pipe2[0];
            let ret = unsafe { ::libc::fcntl(fd, ::libc::F_SETPIPE_SZ, buf_size) };
            if ret == -1 {
                return Err(io::Error::last_os_error());
            };
        }

        let [rx, tx] = pipe2;
        Ok(Self {
            tx: Pipe {
                kind: PipeKind::TX,
                id,
                fd: unsafe { OwnedFd::from_raw_fd(tx) },
                buf_size,
                state: PipeState::UNLIMITED,
            },
            rx: Pipe {
                kind: PipeKind::RX,
                id,
                fd: unsafe { OwnedFd::from_raw_fd(rx) },
                buf_size,
                state: PipeState::UNLIMITED,
            },
        })
    }
}

impl<FD> Pipe2<FD>
where
    FD: AsRawFd,
{
    #[inline]
    fn as_ptr(&self) -> Pipe2 {
        Pipe2 {
            tx: self.tx.as_ptr(),
            rx: self.rx.as_ptr(),
        }
    }
}
#[derive(Debug)]
struct Pipe2Pin {
    data: Pipe2<OwnedFd>,
    refcnt: u8,
}

pub(crate) struct Pipe2Pool {
    buf_size: i32,
    pool: HashMap<PipeID, Pipe2Pin>,
    queue: VecDeque<PipeID>,
    seed: PipeID,
}

impl Pipe2Pool {
    pub(crate) fn new(buf_size: i32) -> Self {
        Self {
            buf_size,
            pool: Default::default(),
            queue: Default::default(),
            seed: Default::default(),
        }
    }

    pub(crate) fn dequeue(&mut self) -> io::Result<Pipe2> {
        match self.queue.pop_back() {
            Some(id) => {
                let pipe = self.pool.get_mut(&id).expect("missing pipe2");
                pipe.refcnt += 2;
                Ok(pipe.data.as_ptr())
            }
            None => {
                let id = self.seed.next();
                let pin = Pipe2Pin {
                    data: Pipe2::new(id, self.buf_size)?,
                    refcnt: 2,
                };
                let ptr = pin.data.as_ptr();
                self.pool.insert(id, pin);
                Ok(ptr)
            }
        }
    }

    pub(crate) fn enqueue(&mut self, pipe: Pipe) {
        let id = pipe.id;
        let pin = self.pool.get_mut(&id).expect("missing pipe2");
        if pipe.kind.tx {
            pin.data.tx.update(pipe);
            pin.refcnt -= 1;
        }
        if pipe.kind.rx {
            pin.data.rx.update(pipe);
            pin.refcnt -= 1;
        }
        if pin.refcnt == 0 {
            self.queue.push_back(id);
        }
    }
}
