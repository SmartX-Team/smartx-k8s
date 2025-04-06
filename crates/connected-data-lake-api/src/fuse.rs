use std::{borrow::Borrow, ffi::OsStr, os::unix::ffi::OsStrExt, path::Path, time::Duration};

use fuser::{ReplyAttr, ReplyData, ReplyEntry, Request};
use libc::ENOENT;

use crate::fs::{Filesystem, FilesystemMetadata};

pub struct FuseFilesystem<T> {
    fs: T,
    ttl: Duration,
}

impl<T> FuseFilesystem<T> {
    pub(crate) fn new(fs: T) -> Self
    where
        T: FilesystemMetadata,
    {
        let ttl = fs.ttl();
        Self { fs, ttl }
    }
}

impl<T> ::fuser::Filesystem for FuseFilesystem<T>
where
    T: Filesystem + FilesystemMetadata<Inode = u64>,
    <T as FilesystemMetadata>::Path: AsRef<Path>,
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        match name.to_str() {
            Some(name) => match self.fs.lookup(parent, name) {
                Ok(attr) => reply.entry(&self.ttl, &attr, 0),
                Err(error) => {
                    #[cfg(feature = "tracing")]
                    {
                        ::tracing::error!("lookup(parent: {parent:#x?}, name {name:?}): {error}")
                    };
                    let _ = error;
                    reply.error(ENOENT)
                }
            },
            None => reply.error(ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        match self.fs.getattr(ino) {
            Ok(attr) => reply.attr(&self.ttl, &attr),
            Err(error) => {
                #[cfg(feature = "tracing")]
                {
                    ::tracing::error!("getattr(ino: {ino:#x?}, fh: {fh:#x?}): {error}")
                };
                let _ = error;
                reply.error(ENOENT)
            }
        }
    }

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyData) {
        match self.fs.readlink(ino) {
            Ok(path) => reply.data(path.borrow().as_ref().as_os_str().as_bytes()),
            Err(error) => {
                #[cfg(feature = "tracing")]
                {
                    ::tracing::error!("readlink(ino: {ino:#x?}): {error}")
                };
                let _ = error;
                reply.error(ENOENT)
            }
        }
    }

    // fn readdir(
    //     &mut self,
    //     _req: &Request,
    //     ino: u64,
    //     _fh: u64,
    //     offset: i64,
    //     mut reply: ReplyDirectory,
    // ) {
    //     let inode_col = self
    //         .inode_table
    //         .column_by_name("inode")
    //         .unwrap()
    //         .as_any()
    //         .downcast_ref::<UInt64Array>()
    //         .unwrap();
    //     let parent_col = self
    //         .inode_table
    //         .column_by_name("parent")
    //         .unwrap()
    //         .as_any()
    //         .downcast_ref::<UInt64Array>()
    //         .unwrap();
    //     let name_col = self
    //         .inode_table
    //         .column_by_name("name")
    //         .unwrap()
    //         .as_any()
    //         .downcast_ref::<StringArray>()
    //         .unwrap();
    //     let is_dir_col = self
    //         .inode_table
    //         .column_by_name("is_directory")
    //         .unwrap()
    //         .as_any()
    //         .downcast_ref::<BooleanArray>()
    //         .unwrap();

    //     let mut entries = vec![
    //         (ino, FileType::Directory, "."),
    //         (1, FileType::Directory, ".."),
    //     ];

    //     for i in 0..self.inode_table.num_rows() {
    //         if parent_col.value(i) == ino {
    //             let kind = if is_dir_col.value(i) {
    //                 FileType::Directory
    //             } else {
    //                 FileType::RegularFile
    //             };
    //             entries.push((inode_col.value(i), kind, name_col.value(i)));
    //         }
    //     }

    //     for (i, (inode, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
    //         reply.add(inode, (i + 1) as i64, kind, name);
    //     }
    //     reply.ok();
    // }

    // fn read(
    //     &mut self,
    //     _req: &Request,
    //     ino: u64,
    //     _fh: u64,
    //     offset: i64,
    //     size: u32,
    //     reply: ReplyData,
    // ) {
    //     match self.0.read(path, buf)
    //     let block_ids_col = self
    //         .inode_table
    //         .column_by_name("block_ids")
    //         .unwrap()
    //         .as_any()
    //         .downcast_ref::<ListArray>()
    //         .unwrap();
    //     let block_val_col = block_ids_col
    //         .values()
    //         .as_any()
    //         .downcast_ref::<UInt64Array>()
    //         .unwrap();
    //     if let Some(i) = self.get_inode_by_ino(ino) {
    //         let block_slice = block_ids_col.value(i);
    //         let block_ids: Vec<u64> = (block_slice.offset()
    //             ..block_slice.offset() + block_slice.len())
    //             .map(|j| block_val_col.value(j as usize))
    //             .collect();

    //         let full_data = self.get_block_data(&block_ids);
    //         let start = offset as usize;
    //         let end = (start + size as usize).min(full_data.len());
    //         reply.data(&full_data[start..end]);
    //     } else {
    //         reply.error(ENOENT);
    //     }
    // }
}
