pub mod block;
pub mod error;
pub mod fs;
#[cfg(feature = "fuse")]
pub mod fuse;

pub mod types {
    #[cfg(feature = "arrow")]
    use arrow::datatypes::{DataType, TimeUnit};

    pub type BlockDeviceIndex = u64;

    #[cfg(feature = "arrow")]
    pub const BLOCK_DEVICE_INDEX: DataType = DataType::UInt64;
    #[cfg(feature = "arrow")]
    pub const BLOCK_DEVICE_DATA: DataType = DataType::Binary;

    pub type InodeIndex = u64;

    #[cfg(feature = "arrow")]
    pub const INODE_INDEX: DataType = DataType::UInt64;

    #[cfg(feature = "arrow")]
    pub const TIMESPEC: DataType = DataType::Time64(TimeUnit::Nanosecond);
}
