use std::{io, mem::MaybeUninit, ptr};

use io_uring::types::{self, Fd};

#[derive(Debug)]
pub struct Signal {
    pub(super) fd: Fd,
    pub(super) buf: MaybeUninit<::libc::signalfd_siginfo>,
}

impl Signal {
    pub fn new() -> Result<Self, io::Error> {
        Ok(Self {
            fd: types::Fd(unsafe {
                let mut mask = MaybeUninit::<::libc::sigset_t>::uninit();
                ::libc::sigemptyset(mask.as_mut_ptr());
                let mut mask = mask.assume_init();

                ::libc::sigaddset(&mut mask, ::libc::SIGINT);
                ::libc::sigaddset(&mut mask, ::libc::SIGTERM);

                ::libc::sigprocmask(::libc::SIG_BLOCK, &mask, ptr::null_mut());

                let flags = ::libc::SFD_CLOEXEC | ::libc::SFD_NONBLOCK;
                ::libc::signalfd(-1, &mask, flags)
            }),
            buf: MaybeUninit::zeroed(),
        })
    }
}
