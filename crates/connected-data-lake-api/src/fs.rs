use std::time::Duration;

use async_trait::async_trait;
#[cfg(feature = "fuse")]
use fuser::FileAttr;

use crate::error::Result;
#[cfg(feature = "fuse")]
use crate::fuse::FuseFilesystem;

pub trait FilesystemMetadata {
    type Inode;
    type Path: ?Sized;

    /// Return the TTL of fuse filesystem.
    ///
    /// Default to 1 second.
    ///
    #[cfg(feature = "fuse")]
    fn ttl(&self) -> Duration {
        Duration::from_secs(1)
    }
}

#[async_trait]
pub trait Filesystem
where
    Self: FilesystemMetadata,
{
    #[cfg(feature = "fuse")]
    async fn lookup(
        &self,
        parent: <Self as FilesystemMetadata>::Inode,
        name: &str,
    ) -> Result<FileAttr>;

    #[cfg(feature = "fuse")]
    async fn getattr(&self, ino: <Self as FilesystemMetadata>::Inode) -> Result<FileAttr>;

    async fn read(
        &self,
        path: &<Self as FilesystemMetadata>::Path,
        buf: &mut [u8],
    ) -> Result<usize>;

    async fn write(&self, path: &<Self as FilesystemMetadata>::Path, buf: &[u8]) -> Result<usize>;

    #[cfg(feature = "fuse")]
    #[inline]
    fn try_into_fuse(self) -> Result<FuseFilesystem<Self>>
    where
        Self: Sized,
    {
        FuseFilesystem::try_new(self)
    }
}

#[async_trait]
pub trait FilesystemExt
where
    Self: Filesystem,
{
    #[cfg(feature = "fuse")]
    #[inline]
    fn try_into_fuse(self) -> Result<FuseFilesystem<Self>>
    where
        Self: Sized,
    {
        FuseFilesystem::try_new(self)
    }
}

#[async_trait]
impl<T> FilesystemExt for T where Self: Filesystem {}
