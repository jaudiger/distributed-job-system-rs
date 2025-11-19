#[allow(unused, clippy::struct_field_names)]
pub struct Operation {
    job_id: String,
    operation_id: String,
    request: String,
    result: String,
}

impl Operation {
    pub fn new(
        job_id: impl Into<String>,
        operation_id: impl Into<String>,
        request: impl Into<String>,
        result: impl Into<String>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            operation_id: operation_id.into(),
            request: request.into(),
            result: result.into(),
        }
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    #[allow(unused)]
    pub fn request(&self) -> &str {
        &self.request
    }

    pub fn result(&self) -> &str {
        &self.result
    }
}
