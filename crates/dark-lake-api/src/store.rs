use std::{ops, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::src::Source;

pub struct CachedBytes {
    pub created_at: DateTime<Utc>,
    data: Vec<u8>,
}

impl AsRef<[u8]> for CachedBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl ops::Deref for CachedBytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub trait Store {
    fn store(&self, data: &[u8]) -> Result<()>;

    #[inline]
    fn replay(&self) -> Result<Option<Source>> {
        Ok(None)
    }

    fn rewind(&self) -> Result<Source>;
}

pub type DynStore = Arc<dyn Store>;
