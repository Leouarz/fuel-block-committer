use avail_rust::H256;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailDispersalStatus {
    Processing,
    Confirmed,
    Finalized,
    Failed,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailDASubmission {
    pub id: Option<u64>,
    pub tx_hash: H256,
    pub tx_id: u32,
    pub block_hash: H256,
    pub block_number: u32,
    pub created_at: Option<DateTime<Utc>>,
    pub status: AvailDispersalStatus,
}

impl Default for AvailDASubmission {
    fn default() -> Self {
        Self {
            id: None,
            tx_hash: H256::zero(),
            tx_id: 0,
            block_hash: H256::zero(),
            block_number: 0,
            status: AvailDispersalStatus::Processing,
            created_at: None,
        }
    }
}
