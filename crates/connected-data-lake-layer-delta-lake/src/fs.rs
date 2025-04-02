use std::{collections::BTreeMap, path::Path, sync::Arc};

use async_trait::async_trait;
use connected_data_lake_api::{
    block::BlockDevice,
    error::{Error, Result},
    fs::FilesystemMetadata,
};
use datafusion::{catalog::TableProvider, datasource::MemTable};
#[cfg(feature = "fuse")]
use fuser::FileAttr;
use tokio::sync::Mutex;

use crate::schema::{schema_inode, types};

#[derive(Debug)]
struct File {
    #[cfg(feature = "fuse")]
    attr: FileAttr,
    blocks: Vec<types::BlockDeviceIndex>,
    children: BTreeMap<String, types::InodeIndex>,
    parent: Option<types::InodeIndex>,
}

pub struct Filesystem<B> {
    blocks: B,
    staging: Mutex<BTreeMap<types::InodeIndex, File>>,
    table: Arc<dyn TableProvider>,
}

impl Filesystem<crate::block::BlockDevice> {
    pub fn new_inmemory() -> Result<Self> {
        let schema = Arc::new(schema_inode());
        let table = Arc::new(MemTable::try_new(schema, vec![vec![]])?);

        // Create root directory
        // FIXME: Create root directory

        Ok(Self {
            blocks: crate::block::BlockDevice::new_inmemory()?,
            staging: Default::default(),
            table,
        })
    }
}

impl<B> FilesystemMetadata for Filesystem<B> {
    type Inode = types::InodeIndex;

    type Path = Path;
}

#[async_trait]
impl<B> ::connected_data_lake_api::fs::Filesystem for Filesystem<B>
where
    B: Sync + BlockDevice,
{
    #[cfg(feature = "fuse")]
    async fn lookup(
        &self,
        parent: <Self as FilesystemMetadata>::Inode,
        name: &str,
    ) -> Result<FileAttr> {
        let staging = self.staging.lock().await;
        staging
            .get(&parent)
            .and_then(|file| file.children.get(name))
            .and_then(|ino| staging.get(ino))
            .map(|file| file.attr)
            .ok_or(Error::NotFound)
    }

    #[cfg(feature = "fuse")]
    async fn getattr(&self, ino: <Self as FilesystemMetadata>::Inode) -> Result<FileAttr> {
        let staging = self.staging.lock().await;
        staging
            .get(&ino)
            .map(|file| file.attr)
            .ok_or(Error::NotFound)
    }

    async fn read(
        &self,
        path: &<Self as FilesystemMetadata>::Path,
        buf: &mut [u8],
    ) -> Result<usize> {
        todo!()
    }

    async fn write(&self, path: &<Self as FilesystemMetadata>::Path, buf: &[u8]) -> Result<usize> {
        todo!()
    }
}
