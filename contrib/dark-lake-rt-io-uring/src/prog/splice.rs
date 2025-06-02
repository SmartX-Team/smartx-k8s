use crate::pipe::Pipe;

#[derive(Copy, Clone, Debug)]
pub struct Splice {
    pub rx: Pipe,
    pub tx: Pipe,
    pub remaining: i64,
}
