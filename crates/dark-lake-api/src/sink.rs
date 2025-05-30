use std::os::fd::RawFd;

pub enum Sink {
    Memory(Vec<u8>),
    RawFd(RawFd),
}
