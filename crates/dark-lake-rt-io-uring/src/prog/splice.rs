use io_uring::{squeue::Flags, types::Fd};

#[derive(Copy, Clone, Debug)]
pub struct Splice {
    pub fd_in: Fd,
    pub fd_out: Fd,
    pub off_in: i64,
    pub off_out: i64,
    pub buf: i32,
    pub remaining: i64,
    pub flags: Flags,
}
