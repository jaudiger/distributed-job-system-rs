use crate::domain;
use core::str::FromStr;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    job_id: String,
    operation_id: String,
    request: String,
}

impl OperationRequest {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn request(&self) -> &str {
        &self.request
    }
}

impl FromStr for OperationRequest {
    type Err = anyhow::Error;

    fn from_str(message: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Self>(message).map_err(|err| anyhow::anyhow!(err))
    }
}

#[derive(serde::Serialize)]
pub struct OperationResult {
    job_id: String,
    operation_id: String,
    result: String,
}

impl From<domain::operation::Operation> for OperationResult {
    fn from(operation: domain::operation::Operation) -> Self {
        Self {
            job_id: operation.job_id().to_string(),
            operation_id: operation.operation_id().to_string(),
            result: operation.result().to_string(),
        }
    }
}
