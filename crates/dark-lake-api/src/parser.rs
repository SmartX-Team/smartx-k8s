use anyhow::Result;

use crate::vm::VirtualMachine;

#[derive(Default)]
pub(crate) struct Parser;

impl Parser {
    pub(crate) fn parse(&self, expr: &str) -> Result<VirtualMachine> {
        todo!()
    }
}
