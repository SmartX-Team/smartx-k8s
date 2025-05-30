use std::sync::Arc;

pub struct Schema {}

pub trait Table {}

pub type DynTable = Arc<dyn Table>;
