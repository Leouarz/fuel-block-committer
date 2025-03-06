use avail_rust::{prelude::ClientError, Block, Filter, Keypair, SecretUri, SDK as AvailSDK};
use core::str::FromStr;
use services::{
    types::{AvailDASubmission, AvailDispersalStatus, Fragment},
    Error as ServiceError, Result as ServiceResult,
};
use url::Url;

#[derive(Debug, Clone)]
pub struct AvailDAClient {
    client: AvailSDK,
    signer: Keypair,
}

impl AvailDAClient {
    pub async fn new(key: String, rpc: Url) -> Result<Self, ClientError> {
        let sdk = AvailSDK::new(&rpc.to_string()).await?;
        let secret_uri = SecretUri::from_str(&key)?;
        let account = Keypair::from_uri(&secret_uri)?;

        Ok(Self {
            client: sdk,
            signer: account,
        })
    }
}

impl services::state_committer::port::avail_da::Api for AvailDAClient {
    async fn submit_state_fragment(&self, fragment: Fragment) -> ServiceResult<AvailDASubmission> {
        let data: Vec<_> = fragment.data.into_iter().collect();
        let options = avail_rust::Options::new().app_id(0);
        let tx = self.client.tx.data_availability.submit_data(data);

        let res = tx
            .execute_and_watch_inclusion(&self.signer, options)
            .await
            .map_err(|e| ServiceError::Other(format!("Failed to send blob on Avail: {e:?}")))?;

        if res.is_successful() != Some(true) {
            return Err(ServiceError::Other(format!(
                "Avail Transaction was not successful"
            )));
        }

        println!(
            "AAAAAAAAAAAAHHHHHHHHHHH Block Hash: {:?}, Block Number: {}, Tx Hash: {:?}, Tx Index: {}",
            res.block_hash, res.block_number, res.tx_hash, res.tx_index
        );

        Ok(AvailDASubmission {
            tx_hash: res.tx_hash,
            tx_id: res.tx_index,
            block_hash: res.block_hash,
            block_number: res.block_number,
            created_at: None,
            status: AvailDispersalStatus::Processing,
            ..Default::default()
        })
    }
}

impl services::state_listener::port::avail_da::Api for AvailDAClient {
    fn get_blob_status(
        &self,
        avail_submission: &AvailDASubmission,
    ) -> impl ::core::future::Future<Output = ServiceResult<AvailDispersalStatus>> + Send {
        // TODO AVAIL we can use transaction_state to simplify this and simplify AvailDASubmission so it only has tx_hash.
        async move {
            // Get finalized block number for canonical chain checking.
            let finalized_block_number = self
                .client
                .client
                .finalized_block_number()
                .await
                .map_err(|e| {
                    ServiceError::Other(format!("Finalized block number fetch error: {:?}", e))
                })?;

            // We use the block number to get the block so we can avoid checking for forks.
            let block =
                Block::from_block_number(&self.client.client, avail_submission.block_number)
                    .await
                    .map_err(|e| {
                        ServiceError::Other(format!("Block fetch by number error: {:?}", e))
                    })?;

            let blobs: Vec<avail_rust::prelude::DataSubmission> =
                block.data_submissions(Filter::new().tx_hash(avail_submission.tx_hash));
            let mut found = !blobs.is_empty();
            let mut finalized = finalized_block_number >= avail_submission.block_number;

            // If we have not found our transaction, it may have been submitted in one of the next blocks due to a fork.
            // We check for 5
            if !found {
                for i in 1..=5 {
                    let next_number = avail_submission.block_number + i;
                    if let Ok(next_block) =
                        Block::from_block_number(&self.client.client, next_number).await
                    {
                        if next_block
                            .data_submissions(Filter::new().tx_hash(avail_submission.tx_hash))
                            .len()
                            > 0
                        {
                            found = true;
                            finalized = finalized_block_number >= next_block.block.number();
                            break;
                        }
                    }
                }
            }

            let new_status = match (found, finalized) {
                (true, true) => AvailDispersalStatus::Finalized,
                (true, false) => AvailDispersalStatus::Confirmed,
                _ => AvailDispersalStatus::Failed,
            };

            println!(
                "AAAAAAAAAAAAHHHHHHHHHHH Updating state for tx: {:?} - new status: {:?}",
                avail_submission.tx_hash, new_status
            );

            Ok(new_status)
        }
    }
}
