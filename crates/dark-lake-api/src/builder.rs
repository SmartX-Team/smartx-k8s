use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::kernel::Kind;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Parameters {
    #[serde(rename = "spec")]
    pub attributes: Map<String, Value>,
    pub secrets: Map<String, Value>,
}

pub trait Builder<T> {
    fn kind(&self) -> Kind;

    fn name(&self) -> String;

    fn build(&self, params: Parameters) -> Result<T>;
}
