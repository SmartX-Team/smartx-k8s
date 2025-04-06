use std::{collections::BTreeMap, path::Path};

use connected_data_lake_api::{
    block::BlockDevice,
    error::{Error, Result},
    fs::FilesystemMetadata,
    types,
};
#[cfg(feature = "fuse")]
use fuser::FileAttr;

#[derive(Debug)]
struct File {
    #[cfg(feature = "fuse")]
    attr: FileAttr,
    blocks: Vec<types::BlockDeviceIndex>,
    children: BTreeMap<String, types::InodeIndex>,
    parent: Option<types::InodeIndex>,
}

#[derive(Debug, Default)]
pub struct Filesystem<B = crate::block::BlockDevice> {
    blocks: B,
    staging: BTreeMap<types::InodeIndex, File>,
}

impl<B> FilesystemMetadata for Filesystem<B> {
    type Inode = types::InodeIndex;

    type Path = Path;
}

impl<B> ::connected_data_lake_api::fs::Filesystem for Filesystem<B>
where
    B: Send + BlockDevice,
{
    #[cfg(feature = "fuse")]
    fn lookup(
        &mut self,
        parent: <Self as FilesystemMetadata>::Inode,
        name: &str,
    ) -> Result<FileAttr> {
        self.staging
            .get(&parent)
            .and_then(|file| file.children.get(name))
            .and_then(|ino| self.staging.get(ino))
            .map(|file| file.attr)
            .ok_or(Error::NotFound)
    }

    #[cfg(feature = "fuse")]
    fn getattr(&mut self, ino: <Self as FilesystemMetadata>::Inode) -> Result<FileAttr> {
        self.staging
            .get(&ino)
            .map(|file| file.attr)
            .ok_or(Error::NotFound)
    }

    #[cfg(feature = "fuse")]
    fn readlink(
        &mut self,
        ino: <Self as FilesystemMetadata>::Inode,
    ) -> Result<<<Self as FilesystemMetadata>::Path as ToOwned>::Owned> {
        todo!()
    }
}
