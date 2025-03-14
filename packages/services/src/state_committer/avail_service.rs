use crate::types::CollectNonEmpty;
use crate::{Error as ServiceError, Result, Runner, types::storage::BundleFragment};
use itertools::Itertools;
use metrics::{
    RegistersMetrics,
    prometheus::{IntGauge, Opts, core::Collector},
};
use nonempty::NonEmpty;
use tracing::info;

use super::commit_helpers::update_current_height_to_commit_metric;

// src/config.rs
#[derive(Debug, Clone)]
pub struct Config {
    // The throughput of the da layer API in MB/s.
    pub api_throughput: u32,
    /// The lookback window in blocks to determine the starting height.
    pub lookback_window: u32,
}

#[cfg(feature = "test-helpers")]
impl Default for Config {
    fn default() -> Self {
        Self {
            api_throughput: 16,
            lookback_window: 1000,
        }
    }
}

struct Metrics {
    current_height_to_commit: IntGauge,
}

impl Default for Metrics {
    fn default() -> Self {
        let current_height_to_commit = IntGauge::with_opts(Opts::new(
            "current_height_to_commit",
            "The starting l2 height of the bundle we're committing/will commit next",
        ))
        .expect("metric config to be correct");

        Self {
            current_height_to_commit,
        }
    }
}

impl<DALayer, FuelApi, Db, Clock> RegistersMetrics for StateCommitter<DALayer, FuelApi, Db, Clock> {
    fn metrics(&self) -> Vec<Box<dyn Collector>> {
        vec![Box::new(self.metrics.current_height_to_commit.clone())]
    }
}

/// The `StateCommitter` is responsible for committing state fragments to L1.
pub struct StateCommitter<DALayer, FuelApi, Db, Clock> {
    da_layer: DALayer,
    fuel_api: FuelApi,
    storage: Db,
    config: Config,
    clock: Clock,
    metrics: Metrics,
}

impl<DALayer, FuelApi, Db, Clock> StateCommitter<DALayer, FuelApi, Db, Clock>
where
    Clock: crate::state_committer::port::Clock,
{
    /// Creates a new `StateCommitter`.
    pub fn new(
        da_layer: DALayer,
        fuel_api: FuelApi,
        storage: Db,
        config: Config,
        clock: Clock,
    ) -> Self {
        Self {
            da_layer,
            fuel_api,
            storage,
            config,
            clock,
            metrics: Metrics::default(),
        }
    }
}

impl<DALayer, FuelApi, Db, Clock> StateCommitter<DALayer, FuelApi, Db, Clock>
where
    DALayer: crate::state_committer::port::avail_da::Api + Send + Sync,
    FuelApi: crate::state_committer::port::fuel::Api,
    Db: crate::state_committer::port::Storage,
    Clock: crate::state_committer::port::Clock,
{
    async fn submit_fragments(&self, fragments: NonEmpty<BundleFragment>) -> Result<()> {
        info!(
            "about to send at most {} fragments, first fragment size: {}",
            fragments.len(),
            fragments.head.fragment.data.len()
        );

        let data = fragments.clone().map(|f| f.fragment);

        match self.da_layer.submit_state_fragments(data).await {
            Ok(submitted_txs) => {
                let fragment_ids = fragments
                    .iter()
                    .map(|f| f.id)
                    .take(submitted_txs.len())
                    .collect_nonempty();

                if let Some(fragment_ids) = fragment_ids {
                    for (submitted_tx, fragment_id) in submitted_txs.into_iter().zip(fragment_ids) {
                        let tx_hash = submitted_tx.tx_hash.clone();
                        self.storage
                            .record_availda_submission(
                                submitted_tx,
                                fragment_id.as_i32(),
                                self.clock.now(),
                            )
                            .await?;
                        tracing::info!(
                            "Submitted fragment {:?} with tx {:?}",
                            fragment_id,
                            tx_hash
                        );
                    }
                    Ok(())
                } else {
                    let ids = fragments
                        .iter()
                        .map(|f| f.id.as_u32().to_string())
                        .join(", ");
                    tracing::error!("Failed to submit fragments {ids}: submitted_txs was empty");
                    Err(ServiceError::Other(format!(
                        "Failed to submit fragments {ids}: submitted_txs was empty"
                    )))
                }
            }
            Err(e) => {
                let ids = fragments
                    .iter()
                    .map(|f| f.id.as_u32().to_string())
                    .join(", ");
                tracing::error!("Failed to submit fragments {ids}: {e}");
                Err(e)
            }
        }
    }

    fn update_oldest_block_metric(&self, oldest_height: u32) {
        self.metrics
            .current_height_to_commit
            .set(oldest_height.into());
    }

    async fn next_fragments_to_submit(&self) -> Result<Option<NonEmpty<BundleFragment>>> {
        let latest_height = self.fuel_api.latest_height().await?;
        let starting_height = latest_height.saturating_sub(self.config.lookback_window);

        let existing_fragments = self
            .storage
            .oldest_unsubmitted_fragments(starting_height, 10) // TODO Avail 40
            .await?;

        println!(
            "Getting next fragment to submit: length: {:?}",
            existing_fragments.len()
        );

        let fragments = NonEmpty::collect(existing_fragments);

        if let Some(fragments) = fragments.as_ref() {
            self.update_oldest_block_metric(
                fragments
                    .minimum_by_key(|b| b.oldest_block_in_bundle)
                    .oldest_block_in_bundle,
            );
        }

        Ok(fragments)
    }

    async fn should_submit(&self, _fragments: &NonEmpty<BundleFragment>) -> Result<bool> {
        // Transaction pool handles this part.
        Ok(true)
    }

    async fn submit_fragments_if_ready(&self, fragments: NonEmpty<BundleFragment>) -> Result<()> {
        if self.should_submit(&fragments).await? {
            self.submit_fragments(fragments).await?;
        }

        Ok(())
    }

    async fn update_current_height_to_commit_metric(&self) -> Result<()> {
        update_current_height_to_commit_metric(
            &self.fuel_api,
            &self.storage,
            self.config.lookback_window,
            &self.metrics.current_height_to_commit,
        )
        .await
    }
}

impl<DALayer, FuelApi, Db, Clock> Runner for StateCommitter<DALayer, FuelApi, Db, Clock>
where
    DALayer: crate::state_committer::port::avail_da::Api + Send + Sync,
    FuelApi: crate::state_committer::port::fuel::Api + Send + Sync,
    Db: crate::state_committer::port::Storage + Clone + Send + Sync,
    Clock: crate::state_committer::port::Clock + Send + Sync,
{
    async fn run(&mut self) -> Result<()> {
        if let Some(fragments) = self.next_fragments_to_submit().await? {
            self.submit_fragments_if_ready(fragments).await?;
        } else {
            self.update_current_height_to_commit_metric().await?;
        };

        Ok(())
    }
}
