use avail_rust::H256;
use services::types::{AvailDispersalStatus, DateTime, Utc};

#[derive(sqlx::FromRow)]
pub struct AvailDASubmission {
    pub id: i32,
    pub tx_hash: Vec<u8>,
    pub block_number: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub status: i16,
}

macro_rules! bail {
    ($msg: literal, $($args: expr),*) => {
        return Err($crate::error::Error::Conversion(format!($msg, $($args),*)))
    };
}

impl AvailDASubmission {
    pub fn parse_status(&self) -> Result<AvailDispersalStatus, crate::error::Error> {
        match self.status {
            0 => Ok(AvailDispersalStatus::Processing),
            1 => Ok(AvailDispersalStatus::Finalized),
            2 => Ok(AvailDispersalStatus::Failed),
            3 => Ok(AvailDispersalStatus::Confirmed),
            _ => {
                bail!(
                    "AvailDASubmission(id={}) has invalid status {}",
                    self.id,
                    self.status
                )
            }
        }
    }
}

impl From<services::types::AvailDASubmission> for AvailDASubmission {
    fn from(value: services::types::AvailDASubmission) -> Self {
        let status = AvailSubmissionStatus::from(value.status).into();

        Self {
            // if not present use placeholder as id is given by db
            id: value.id.unwrap_or_default() as i32,
            tx_hash: value.tx_hash.as_bytes().to_vec(),
            block_number: value.block_number as i32,
            status,
            created_at: value.created_at,
        }
    }
}

impl TryFrom<AvailDASubmission> for services::types::AvailDASubmission {
    type Error = crate::error::Error;

    fn try_from(value: AvailDASubmission) -> Result<Self, Self::Error> {
        let status = value.parse_status()?;

        let id = value.id.try_into().map_err(|_| {
            Self::Error::Conversion(format!(
                "Could not convert `id` to u64. Got: {} from db",
                value.id
            ))
        })?;

        Ok(Self {
            id: Some(id),
            tx_hash: H256::from_slice(&value.tx_hash),
            block_number: value.block_number as u32,
            status,
            created_at: value.created_at,
        })
    }
}

pub enum AvailSubmissionStatus {
    Processing,
    Confirmed,
    Finalized,
    Failed,
}

impl From<AvailDispersalStatus> for AvailSubmissionStatus {
    fn from(status: AvailDispersalStatus) -> Self {
        match status {
            AvailDispersalStatus::Processing => AvailSubmissionStatus::Processing,
            AvailDispersalStatus::Confirmed => AvailSubmissionStatus::Confirmed,
            AvailDispersalStatus::Finalized => AvailSubmissionStatus::Finalized,
            AvailDispersalStatus::Failed | AvailDispersalStatus::Other(_) => {
                AvailSubmissionStatus::Failed
            }
        }
    }
}

impl From<AvailSubmissionStatus> for i16 {
    fn from(status: AvailSubmissionStatus) -> Self {
        match status {
            AvailSubmissionStatus::Processing => 0,
            AvailSubmissionStatus::Finalized => 1,
            AvailSubmissionStatus::Failed => 2,
            AvailSubmissionStatus::Confirmed => 3,
        }
    }
}
