use metrics::{
    prometheus::{core::Collector, IntGauge, Opts},
    RegistersMetrics,
};

use crate::{
    types::{AvailDASubmission, AvailDispersalStatus},
    Runner,
};

pub struct StateListener<AvailDA, Db> {
    availda_adapter: AvailDA,
    storage: Db,
    metrics: Metrics,
}

impl<AvailDA, Db> StateListener<AvailDA, Db> {
    pub fn new(l1_adapter: AvailDA, storage: Db, last_finalization_time_metric: IntGauge) -> Self {
        Self {
            availda_adapter: l1_adapter,
            storage,
            metrics: Metrics::new(last_finalization_time_metric),
        }
    }
}

impl<AvailDA, Db> StateListener<AvailDA, Db>
where
    AvailDA: crate::state_listener::port::avail_da::Api,
    Db: crate::state_listener::port::Storage,
{
    async fn check_non_finalized(
        &self,
        non_finalized: Vec<AvailDASubmission>,
    ) -> crate::Result<()> {
        let mut changes = Vec::with_capacity(non_finalized.len());

        for submission in non_finalized {
            let status = self
                .availda_adapter
                .get_blob_status(&submission)
                .await?;
            let submission_id = submission.id.expect("submission id to be present") as u32;

            // skip if status didn't change
            if status == submission.status {
                continue;
            }

            match status {
                AvailDispersalStatus::Processing => {
                    tracing::info!("Processing submission with tx hash: {}", submission.tx_hash);
                    continue;
                }
                AvailDispersalStatus::Confirmed => {
                    tracing::info!("Confirmed submission with tx hash: {}", submission.tx_hash);
                    changes.push((submission_id, AvailDispersalStatus::Confirmed));
                }
                AvailDispersalStatus::Finalized => {
                    tracing::info!("Finalized submission with tx hash: {}", submission.tx_hash);
                    changes.push((submission_id, AvailDispersalStatus::Finalized));
                }
                _ => {
                    tracing::info!(
                        "Unexpected status - submission with tx hash: {}",
                        submission.tx_hash
                    );
                    changes.push((submission_id, AvailDispersalStatus::Failed));
                }
            }
        }

        self.storage.update_avail_submissions(changes).await?;

        Ok(())
    }
}

impl<AvailDA, Db> Runner for StateListener<AvailDA, Db>
where
    AvailDA: crate::state_listener::port::avail_da::Api + Send + Sync,
    Db: crate::state_listener::port::Storage,
{
    async fn run(&mut self) -> crate::Result<()> {
        let non_finalized = self.storage.get_non_finalized_avail_submission().await?;

        if non_finalized.is_empty() {
            return Ok(());
        }

        self.check_non_finalized(non_finalized).await?;

        Ok(())
    }
}

#[derive(Clone)]
struct Metrics {
    last_eth_block_w_blob: IntGauge,
    last_finalization_time: IntGauge,
    last_finalization_interval: IntGauge,
}

impl<L1, Db> RegistersMetrics for StateListener<L1, Db> {
    fn metrics(&self) -> Vec<Box<dyn Collector>> {
        vec![
            Box::new(self.metrics.last_eth_block_w_blob.clone()),
            Box::new(self.metrics.last_finalization_time.clone()),
            Box::new(self.metrics.last_finalization_interval.clone()),
        ]
    }
}

impl Metrics {
    fn new(last_finalization_time: IntGauge) -> Self {
        let last_eth_block_w_blob = IntGauge::with_opts(Opts::new(
            "last_eth_block_w_blob",
            "The height of the latest Ethereum block used for state submission.",
        ))
        .expect("last_eth_block_w_blob metric to be correctly configured");

        let last_finalization_interval = IntGauge::new(
            "seconds_from_earliest_submission_to_finalization",
            "The number of seconds from the earliest submission to finalization",
        )
        .expect(
            "seconds_from_earliest_submission_to_finalization gauge to be correctly configured",
        );

        Self {
            last_eth_block_w_blob,
            last_finalization_time,
            last_finalization_interval,
        }
    }
}
