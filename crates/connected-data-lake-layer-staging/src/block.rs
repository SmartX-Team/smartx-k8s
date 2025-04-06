use std::collections::BTreeMap;

use connected_data_lake_api::{
    block::BlockDeviceMetadata,
    error::{Error, Result},
    types,
};

#[derive(Debug, Default)]
pub struct BlockDevice {
    staging: BTreeMap<types::BlockDeviceIndex, Vec<u8>>,
}

impl BlockDeviceMetadata for BlockDevice {
    type Index = types::BlockDeviceIndex;
}

impl ::connected_data_lake_api::block::BlockDevice for BlockDevice {
    fn read_one(
        &mut self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &mut [u8],
    ) -> Result<usize> {
        self.staging
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

    fn write_one(
        &mut self,
        index: <Self as BlockDeviceMetadata>::Index,
        buf: &[u8],
    ) -> Result<usize> {
        let len = buf.len();
        {
            self.staging.insert(index, buf.to_vec());
        }
        Ok(len)
    }
}
