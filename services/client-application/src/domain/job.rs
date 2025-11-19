use mongodb::bson::oid::ObjectId;

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum JobStatusEnum {
    InProgress,
    Completed,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Job {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    operations: usize,
}

impl Job {
    pub const fn new_job(operations: usize) -> Self {
        Self {
            id: None,
            operations,
        }
    }

    pub fn id(&self) -> String {
        self.id.map(ObjectId::to_hex).unwrap_or_default()
    }

    pub const fn operations(&self) -> usize {
        self.operations
    }

    pub const fn status(&self, total_finished: usize) -> JobStatusEnum {
        if total_finished == self.operations {
            JobStatusEnum::Completed
        } else {
            JobStatusEnum::InProgress
        }
    }
}
