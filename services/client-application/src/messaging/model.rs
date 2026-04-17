use crate::domain;
use anyhow::Result;

#[derive(serde::Serialize)]
pub struct OperationRequest {
    job_id: String,
    operation_id: String,
    request: String,
}

impl From<domain::operation::Operation> for OperationRequest {
    fn from(operation: domain::operation::Operation) -> Self {
        Self {
            job_id: operation.job_id().to_string(),
            operation_id: operation.id(),
            request: operation.request().to_string(),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    job_id: String,
    operation_id: String,
    result: String,
}

impl OperationResult {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn result(&self) -> &str {
        &self.result
    }
}

impl TryFrom<&str> for OperationResult {
    type Error = anyhow::Error;

    fn try_from(message: &str) -> Result<Self, Self::Error> {
        serde_json::from_str::<Self>(message).map_err(|err| anyhow::anyhow!(err))
    }
}
