use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use connected_data_lake_api::{
    block::BlockDeviceMetadata,
    error::{Error, Result},
};
use datafusion::{catalog::TableProvider, datasource::MemTable};
use tokio::sync::Mutex;

use crate::schema::{schema_block_device, types};

pub struct BlockDevice {
    staging: Mutex<BTreeMap<types::BlockDeviceIndex, Vec<u8>>>,
    table: Arc<dyn TableProvider>,
}

impl BlockDevice {
    pub fn new_inmemory() -> Result<Self> {
        let schema = Arc::new(schema_block_device());
        let table = Arc::new(MemTable::try_new(schema, vec![vec![]])?);

        Ok(Self {
            staging: Default::default(),
            table,
        })
    }
}

impl BlockDeviceMetadata for BlockDevice {
    type Index = types::BlockDeviceIndex;
}

#[async_trait]
impl ::connected_data_lake_api::block::BlockDevice for BlockDevice {
    async fn read_one(
        &self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &mut [u8],
    ) -> Result<usize> {
        let staging = self.staging.lock().await;
        staging
            .get(&index)
            .map(|data| {
                buf.copy_from_slice(data);
                data.len()
            })
            .ok_or(Error::NotFound)

        // // Register the batch in a DataFusion context
        // let ctx = SessionContext::new();
        // ctx.register_table("blocks", self.table.clone())?;

        // // Run SQL query to filter rows
        // let df = ctx
        //     .sql(&format!("SELECT data FROM blocks WHERE id = {index}"))
        //     .await?;
        // let results = df.collect().await?;

        // // Extract binary data and copy into the buffer
        // match results
        //     .iter()
        //     .flat_map(|batch| {
        //         batch
        //             .column(0)
        //             .as_binary::<types::BlockDeviceDataLength>()
        //             .iter()
        //             .filter_map(identity)
        //     })
        //     .next()
        // {
        //     Some(data) => {
        //         buf.copy_from_slice(data);
        //         Ok(data.len())
        //     }
        //     None => Err(Error::NotFound),
        // }
    }

    async fn write_one(
        &self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &[u8],
    ) -> Result<usize> {
        let len = buf.len();
        {
            let staging = self.staging.lock().await;
            self.staging.lock().await.insert(index, buf.to_vec());
        }
        Ok(len)
    }
}
