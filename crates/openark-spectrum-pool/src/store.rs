use std::{fs::OpenOptions, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use openark_spectrum_api::common::ObjectReference;
use redb::{
    Database, DatabaseError, Error, ReadOnlyTable, TableDefinition,
    backends::{FileBackend, InMemoryBackend},
};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct StoreArgs {
    path: Option<PathBuf>,
}

impl StoreArgs {
    pub(crate) fn build(self) -> Result<Store, DatabaseError> {
        let Self { path } = self;

        let db = match path {
            Some(path) => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(path)?;

                let backend = FileBackend::new(file)?;
                Database::builder().create_with_backend(backend)?
            }
            None => {
                let backend = InMemoryBackend::new();
                Database::builder().create_with_backend(backend)?
            }
        };

        Ok(Store { db })
    }
}

type Key = &'static str;
type Value = Option<String>;

pub struct Store {
    db: Database,
}

impl Store {
    const TABLE: TableDefinition<'static, Key, Value> = TableDefinition::new("pools");

    pub fn read(&self) -> Result<ReadableStore, Error> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(Self::TABLE)?;
        Ok(ReadableStore { table })
    }
}

pub struct ReadableStore {
    table: ReadOnlyTable<Key, Value>,
}

impl ReadableStore {
    pub fn get(&self, key: &ObjectReference) -> Result<Value> {
        self.table
            .get(key.to_string().as_str())
            .map(|option| option.and_then(|guard| guard.value()))
            .map_err(Into::into)
    }
}
