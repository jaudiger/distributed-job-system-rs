use crate::domain;

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

impl TryFrom<&str> for OperationRequest {
    type Error = anyhow::Error;

    fn try_from(message: &str) -> Result<Self, Self::Error> {
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
