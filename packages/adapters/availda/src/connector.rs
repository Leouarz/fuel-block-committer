use avail_rust::{
    AOnlineClient, Client, Keypair, SDK as AvailSDK, SecretUri, Transaction, account,
    da_commitments::DaCommitmentBuilder,
    prelude::ClientError,
    transactions::da::{SubmitDataCall, SubmitDataWithCommitmentsCall},
};
use core::str::FromStr;
use futures::stream::{FuturesOrdered, StreamExt};
use services::{
    Error as ServiceError, Result as ServiceResult,
    types::{AvailDASubmission, AvailDispersalStatus, Fragment, NonEmpty},
};
use std::time::Duration;
use url::Url;

use avail_rust::subxt::backend::rpc::{
    RpcClient,
    reconnecting_rpc_client::{ExponentialBackoff, RpcClient as ReconnectingRpcClient},
};

#[derive(Debug, Clone)]
pub struct AvailDAClient {
    client: AvailSDK,
    signer: Keypair,
}

impl AvailDAClient {
    pub async fn new(key: String, rpc: Url) -> Result<Self, ClientError> {
        let rpc_client = ReconnectingRpcClient::builder()
            .max_request_size(65 * 1024 * 1024)
            .max_response_size(1024 * 1024 * 1024)
            .retry_policy(
                ExponentialBackoff::from_millis(1000)
                    .max_delay(Duration::from_secs(3))
                    .take(3),
            )
            .build(rpc)
            .await
            .map_err(|e| e.to_string())?;

        let rpc_client = RpcClient::new(rpc_client);
        let online_client = AOnlineClient::from_rpc_client(rpc_client.clone()).await?;
        let client = Client::new(online_client, rpc_client);
        let sdk = AvailSDK::new_custom(client).await?;

        let secret_uri = SecretUri::from_str(&key)?;
        let account = Keypair::from_uri(&secret_uri)?;
        AvailSDK::enable_logging();

        Ok(Self {
            client: sdk,
            signer: account,
        })
    }
}

impl services::state_committer::port::avail_da::Api for AvailDAClient {
    async fn submit_state_fragments(
        &self,
        fragments: NonEmpty<Fragment>,
    ) -> ServiceResult<Vec<AvailDASubmission>> {
        let account_id = self.signer.public_key().to_account_id().to_string();
        let nonce = account::nonce(&self.client.client, &account_id)
            .await
            .map_err(|e| {
                ServiceError::Other(format!("Failed to get account nonce on Avail: {e:?}"))
            })?;

        // BIG BLOCKS
        let calls: Vec<Transaction<SubmitDataWithCommitmentsCall>> = fragments
            .into_iter()
            .map(|fragment| {
                let data: Vec<_> = fragment.data.into_iter().collect();
                let commitments = DaCommitmentBuilder::new(data.clone())
                    .build()
                    .expect("DaCommitment Failed on Avail"); // TODO Add better errors !
                self.client
                    .tx
                    .data_availability
                    .submit_data_with_commitments(data, commitments)
            })
            .collect();

        let mut futures = FuturesOrdered::new();
        for (i, tx) in calls.iter().enumerate() {
            let options = avail_rust::Options::new().app_id(0).nonce(nonce + i as u32);
            futures.push_back(tx.execute(&self.signer, options));
        }

        // SMALL BLOCKS
        // let calls: Vec<Transaction<SubmitDataCall>> = fragments
        //     .into_iter()
        //     .map(|fragment| {
        //         let data: Vec<_> = fragment.data.into_iter().collect();
        //         self.client.tx.data_availability.submit_data(data)
        //     })
        //     .collect();

        // let mut futures = FuturesOrdered::new();
        // for (i, tx) in calls.iter().enumerate() {
        //     let options = avail_rust::Options::new().app_id(0).nonce(nonce + i as u32);
        //     futures.push_back(tx.execute(&self.signer, options));
        // }

        let current_block_number = self.client.client.best_block_number().await.map_err(|e| {
            ServiceError::Other(format!("Could not get best block number in Avail: {:?}", e))
        })?;

        let mut submissions = Vec::with_capacity(calls.len());
        while let Some(result) = futures.next().await {
            let tx_hash = result.map_err(|e| {
                ServiceError::Other(format!("Avail Transaction submission failed: {:?}", e))
            })?;

            let submission = AvailDASubmission {
                tx_hash,
                block_number: current_block_number,
                created_at: None,
                status: AvailDispersalStatus::Processing,
                ..Default::default()
            };
            submissions.push(submission);
        }

        Ok(submissions)
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
