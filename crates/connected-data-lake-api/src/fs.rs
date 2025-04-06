use std::time::Duration;

#[cfg(feature = "fuse")]
use fuser::FileAttr;

use crate::error::Result;
#[cfg(feature = "fuse")]
use crate::fuse::FuseFilesystem;

pub trait FilesystemMetadata {
    type Inode;
    type Path: ?Sized + ToOwned;

    /// Return the TTL of fuse filesystem.
    ///
    /// Default to 1 second.
    ///
    #[cfg(feature = "fuse")]
    fn ttl(&self) -> Duration {
        Duration::from_secs(1)
    }
}

pub trait Filesystem
where
    Self: FilesystemMetadata,
{
    #[cfg(feature = "fuse")]
    fn lookup(
        &mut self,
        parent: <Self as FilesystemMetadata>::Inode,
        name: &str,
    ) -> Result<FileAttr>;

    #[cfg(feature = "fuse")]
    fn getattr(&mut self, ino: <Self as FilesystemMetadata>::Inode) -> Result<FileAttr>;

    #[cfg(feature = "fuse")]
    fn readlink(
        &mut self,
        ino: <Self as FilesystemMetadata>::Inode,
    ) -> Result<<<Self as FilesystemMetadata>::Path as ToOwned>::Owned>;
}

pub trait FilesystemExt
where
    Self: Filesystem,
{
    fn read(&mut self, path: &<Self as FilesystemMetadata>::Path, buf: &mut [u8]) -> Result<usize> {
        todo!()
    }

    fn write(&mut self, path: &<Self as FilesystemMetadata>::Path, buf: &[u8]) -> Result<usize> {
        todo!()
    }

    /// Convert filesystem into [`FuseFilesystem`].
    ///
    #[cfg(feature = "fuse")]
    #[inline]
    fn into_fuse(self) -> FuseFilesystem<Self>
    where
        Self: Sized,
    {
        FuseFilesystem::new(self)
    }
}

impl<T> FilesystemExt for T where Self: Filesystem {}
