pub mod block_bundler;
pub mod block_committer;
pub mod block_importer;
pub mod cost_reporter;
pub mod fee_metrics_tracker;
pub mod health_reporter;
pub mod state_committer;
pub mod state_listener;
pub mod state_pruner;
pub mod status_reporter;
pub mod types;
pub mod wallet_balance_tracker;

pub mod fees;

pub use block_bundler::{
    avail_bundler::Factory as AvailBundlerFactory,
    bundler::Factory as BundlerFactory,
    eigen_bundler::Factory as EigenBundlerFactory,
    service::{BlockBundler, Config as BlockBundlerConfig},
};
#[cfg(feature = "test-helpers")]
pub use block_bundler::{
    bundler::Bundler,
    common::{Bundle, BundleProposal, Metadata},
    test_helpers::ControllableBundlerFactory,
};
pub use state_committer::service::{Config as StateCommitterConfig, StateCommitter};
pub use state_committer::{
    avail_service::{Config as AvailStatecommitterConfig, StateCommitter as AvailStateCommitter},
    eigen_service::{Config as EigenStatecommitterConfig, StateCommitter as EigenStateCommitter},
};
use types::InvalidL1Height;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("{0}")]
    Other(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Block validation error: {0}")]
    BlockValidation(String),
}

impl From<InvalidL1Height> for Error {
    fn from(err: InvalidL1Height) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[trait_variant::make(Send)]
pub trait Runner: Sync {
    async fn run(&mut self) -> Result<()>;
}
