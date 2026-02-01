use std::{any::Any, sync::Arc};

use anyhow::Result;
use jiff::Timestamp;

pub trait Layout
where
    Self: Any + Send + Sync,
{
    fn created_at(&self) -> Option<Timestamp>;
}

pub type DynLayout = Box<dyn Layout>;

pub trait Format {
    fn decode(&self, data: &[u8]) -> Result<Option<DynLayout>>;
}

pub type DynFormat = Arc<dyn Format>;
