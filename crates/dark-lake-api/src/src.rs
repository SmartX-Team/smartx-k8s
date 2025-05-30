use std::os::fd::RawFd;

pub enum Source {
    Memory(Vec<u8>),
    RawFd(RawFd),
}
