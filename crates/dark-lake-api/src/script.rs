use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{builder::Parameters, kernel::Kind};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NodeMetadata {
    pub kind: Kind,
    pub model: String,
    pub name: String,
}

impl fmt::Display for NodeMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { kind, model, name } = self;
        write!(f, "{kind}/{model}/{name}")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Node {
    #[serde(flatten)]
    pub metadata: NodeMetadata,

    #[serde(flatten)]
    pub params: Parameters,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Edge<T = String> {
    pub src: T,
    pub sink: T,
}

pub type TypedEdge = Edge<NodeMetadata>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Script {
    #[serde(default)]
    pub nodes: Vec<Node>,

    #[serde(default)]
    pub edges: Vec<Edge>,
}
