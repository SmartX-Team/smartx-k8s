use anyhow::Result;

use crate::script::Script;

#[derive(Default)]
pub(crate) struct Parser;

impl Parser {
    pub(crate) fn parse(&self, expr: &str) -> Result<Script> {
        todo!()
    }
}
