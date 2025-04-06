use arrow::datatypes::{DataType, Field, Schema, SchemaBuilder};
use connected_data_lake_api::types;

/// Append block device schema.
///
fn append_schema_block_device(builder: &mut SchemaBuilder) {
    builder.push(Field::new("id", types::BLOCK_DEVICE_INDEX, false));
    builder.push(Field::new("data", types::BLOCK_DEVICE_DATA, false));
    builder.push(Field::new("atime", types::TIMESPEC, false));
    builder.push(Field::new("mtime", types::TIMESPEC, false));
    builder.push(Field::new("ctime", types::TIMESPEC, false));

    // block size is hidden
}

/// Create a block device schema.
///
#[inline]
pub fn schema_block_device() -> Schema {
    let mut builder = SchemaBuilder::new();
    append_schema_block_device(&mut builder);
    builder.finish()
}

/// Append POSIX inode schema.
///
fn append_schema_inode(builder: &mut SchemaBuilder) {
    builder.push(Field::new("inode", types::INODE_INDEX, false));
    builder.push(Field::new_list(
        "block_ids",
        Field::new("block_id", types::BLOCK_DEVICE_INDEX, false),
        false,
    ));

    builder.push(Field::new("mode", DataType::UInt16, false));
    builder.push(Field::new("opflags", DataType::UInt16, false));
    builder.push(Field::new("uid", DataType::UInt32, false));
    builder.push(Field::new("gid", DataType::UInt32, false));
    builder.push(Field::new("flags", DataType::UInt32, false));

    // POSIX ACL not supported

    // superblock is hidden

    builder.push(Field::new("size", types::BLOCK_DEVICE_INDEX, false));
    builder.push(Field::new("atime", types::TIMESPEC, false));
    builder.push(Field::new("mtime", types::TIMESPEC, false));
    builder.push(Field::new("ctime", types::TIMESPEC, false));

    // lock is hidden
}

/// Create a POSIX inode schema.
///
#[inline]
pub fn schema_inode() -> Schema {
    let mut builder = SchemaBuilder::new();
    append_schema_inode(&mut builder);
    builder.finish()
}
