use avail_rust::{prelude::ClientError, Keypair, SecretUri, SDK as AvailSDK};
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

        let tx_hash = tx
            .execute(&self.signer, options)
            .await
            .map_err(|e| ServiceError::Other(format!("Failed to send blob on Avail: {e:?}")))?;
        let block_number = self.client.client.best_block_number().await.map_err(|e| {
            ServiceError::Other(format!("Failed to get latest block number on Avail: {e:?}"))
        })?;

        Ok(AvailDASubmission {
            tx_hash: tx_hash,
            block_number: block_number,
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
        async move {
            let result = self
                .client
                .client
                .transaction_state(&avail_submission.tx_hash, false)
                .await
                .map_err(|e| {
                    ServiceError::Other(format!("Avail Transaction state query failed: {:?}", e))
                })?;

            let status = if let Some(transaction_state) = result.get(0) {
                match (transaction_state.is_finalized, transaction_state.tx_success) {
                    (true, true) => AvailDispersalStatus::Finalized,
                    (false, true) => AvailDispersalStatus::Confirmed,
                    _ => AvailDispersalStatus::Failed,
                }
            } else {
                // We check the current block number against the block where the submission happened to not consider the transaction failed for a window of time.
                let block_number = self.client.client.best_block_number().await.map_err(|e| {
                    ServiceError::Other(format!(
                        "Failed to get latest block number on Avail: {e:?}"
                    ))
                })?;
                if block_number - avail_submission.block_number <= 5 {
                    AvailDispersalStatus::Processing
                } else {
                    AvailDispersalStatus::Failed
                }
            };

            Ok(status)
        }
    }
}
