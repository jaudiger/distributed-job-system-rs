use mongodb::bson::oid::ObjectId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Operation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    job_id: String,
    request: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}

impl Operation {
    pub fn new_operation(job_id: impl Into<String>, request: impl Into<String>) -> Self {
        Self {
            id: None,
            job_id: job_id.into(),
            request: request.into(),
            result: None,
        }
    }

    pub fn id(&self) -> String {
        self.id.map(ObjectId::to_hex).unwrap_or_default()
    }

    #[allow(unused)]
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn request(&self) -> &str {
        &self.request
    }

    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }
}
