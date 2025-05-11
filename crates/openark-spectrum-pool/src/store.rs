use std::{fs::OpenOptions, path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::Parser;
use openark_spectrum_api::{
    common::ObjectReference,
    pool_claim::PoolResourceProbe,
    schema::{CommitState, PoolResource},
};
use redb::{
    Database, Error, ReadOnlyTable, Table, TableDefinition,
    backends::{FileBackend, InMemoryBackend},
};

use crate::pool::Pool;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct StoreArgs {
    #[arg(long, env = "OPENARK_SPECTRUM_POOL_MAX_SIZE")]
    max_pool: usize,

    #[arg(long, env = "OPENARK_SPECTRUM_POOL_PATH")]
    path: Option<PathBuf>,
}

impl StoreArgs {
    pub(crate) fn build(self) -> Result<Store> {
        let Self { max_pool, path } = self;

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

        // Create tables
        {
            let txn = db.begin_write()?;
            let _ = txn.open_table(Store::TABLE_CLAIM)?;
            let _ = txn.open_table(Store::TABLE_READY)?;
            txn.commit()?
        }

        Ok(Store {
            db: Arc::new(db),
            pool: Pool::new(max_pool)?,
        })
    }
}

type Key = &'static str;
type Value = String;

pub struct Store {
    db: Arc<Database>,
    pool: Pool,
}

impl Store {
    const TABLE_CLAIM: TableDefinition<'static, Key, Value> = TableDefinition::new("claim");
    const TABLE_READY: TableDefinition<'static, Key, u8> = TableDefinition::new("ready");

    pub fn read<F, R>(&self, closure: F) -> Result<R, Error>
    where
        F: FnOnce(&ReadGuard) -> Result<R, Error>,
    {
        let txn = self.db.begin_read()?;
        {
            let table_claim = txn.open_table(Self::TABLE_CLAIM)?;
            let table_ready = txn.open_table(Self::TABLE_READY)?;
            let guard = ReadGuard {
                table_claim,
                table_ready,
            };
            closure(&guard)
        }
    }

    pub fn write<F, R>(&self, closure: F) -> Result<R>
    where
        F: FnOnce(&mut WriteGuard) -> Result<R>,
    {
        let txn = self.db.begin_write()?;
        let output = {
            let table_claim = txn.open_table(Self::TABLE_CLAIM)?;
            let table_ready = txn.open_table(Self::TABLE_READY)?;
            let mut guard = WriteGuard {
                db: &self.db,
                pool: &self.pool,
                table_claim,
                table_ready,
            };
            closure(&mut guard)?
        };
        txn.commit()?;
        Ok(output)
    }
}

pub struct ReadGuard {
    table_claim: ReadOnlyTable<Key, Value>,
    table_ready: ReadOnlyTable<Key, u8>,
}

impl ReadGuard {
    pub fn get(&self, key: &ObjectReference) -> Result<PoolResource, Error> {
        Ok(PoolResource {
            claim: self
                .table_claim
                .get(key.to_string().as_str())
                .map(|option| option.map(|guard| guard.value()))?,
            state: self
                .table_ready
                .get(key.to_string().as_str())
                .map(|option| {
                    option
                        .and_then(|guard| CommitState::from_raw(guard.value()))
                        .unwrap_or_default()
                })?,
        })
    }
}

pub struct WriteGuard<'a> {
    db: &'a Arc<Database>,
    pool: &'a Pool,
    table_claim: Table<'a, Key, Value>,
    table_ready: Table<'a, Key, u8>,
}

impl WriteGuard<'_> {
    pub fn put(
        &mut self,
        key: &ObjectReference,
        value: &str,
        address: &str,
        probe: Option<&PoolResourceProbe>,
    ) -> Result<CommitState> {
        let state = match probe {
            Some(probe) => {
                let on_completed = {
                    let db = self.db.clone();
                    let key = key.to_string();
                    move || {
                        let txn = db.begin_write()?;
                        let mut table = txn.open_table(Store::TABLE_READY)?;
                        table.insert(key.as_str(), CommitState::Running as u8)?;
                        Ok(())
                    }
                };
                self.pool.commit(address, probe, on_completed)?
            }
            None => CommitState::Running,
        };

        let key = key.to_string();
        {
            let value = value.to_string();
            self.table_claim.insert(key.as_str(), value)?;
        }

        {
            self.table_ready.insert(key.as_str(), state as u8)?;
        }
        Ok(state)
    }
}
