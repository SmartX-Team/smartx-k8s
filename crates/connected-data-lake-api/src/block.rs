use async_trait::async_trait;

use crate::error::Result;

/// An old style MBR partition table starts after `512` bytes.
///
const MIN_BLOCK_SIZE: usize = 512;

pub trait BlockDeviceMetadata {
    /// Sector index type.
    ///
    type Index;

    /// Return the unit sector size.
    ///
    /// Default to `8`.
    ///
    fn unit_sector_size(&self) -> usize {
        8
    }
}

pub trait BlockDeviceMetadataExt
where
    Self: BlockDeviceMetadata,
{
    /// Return the internal block size.
    ///
    fn block_size(&self) -> usize {
        self.unit_sector_size() * MIN_BLOCK_SIZE
    }

    /// Return the logical block size.
    ///
    fn logical_block_size(&self) -> usize {
        MIN_BLOCK_SIZE
    }
}

impl<T> BlockDeviceMetadataExt for T where Self: BlockDeviceMetadata {}

#[async_trait]
pub trait BlockDevice
where
    Self: BlockDeviceMetadata,
{
    async fn read_one(
        &self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &mut [u8],
    ) -> Result<usize>;

    async fn write_one(
        &self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &[u8],
    ) -> Result<usize>;
}
