use crate::database;
use crate::domain;
use anyhow::Result;
use common::counter;
use futures::TryStreamExt;
use mongodb::Collection;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;

counter!(
    INSERT_JOB_COUNTER,
    "database_insert_job_requests",
    "Number of insert job requests"
);
counter!(
    DELETE_JOB_COUNTER,
    "database_delete_job_requests",
    "Number of delete job requests"
);
counter!(
    GET_JOB_COUNTER,
    "database_get_job_requests",
    "Number of get job requests"
);
counter!(
    GET_JOBS_COUNTER,
    "database_get_jobs_requests",
    "Number of get jobs requests"
);

pub struct JobRepository {
    collection: Collection<domain::job::Job>,
}

impl JobRepository {
    pub const COLLECTION_NAME: &'static str = "job";

    const ID_FIELD: &'static str = "_id";

    pub fn new(collection: Collection<domain::job::Job>) -> Self {
        tracing::debug!("Initializing the MongoDB job repository");

        Self { collection }
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_job(&self, job: &domain::job::Job) -> Result<String> {
        tracing::debug!("Inserting a job");

        INSERT_JOB_COUNTER.add(1, &[]);

        let result = self.collection.insert_one(job).await?;

        Ok(result
            .inserted_id
            .as_object_id()
            .expect("No ObjectId returned")
            .to_string())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_job(&self, job_id: &str) -> Result<()> {
        tracing::debug!("Deleting job with id {job_id}");

        DELETE_JOB_COUNTER.add(1, &[]);

        let result = self
            .collection
            .delete_one(doc! {Self::ID_FIELD: ObjectId::parse_str(job_id)?})
            .await?;

        if result.deleted_count == 0 {
            anyhow::bail!("Document not found");
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_job(&self, job_id: &str) -> Result<domain::job::Job> {
        tracing::debug!("Getting job with id: {job_id}");

        GET_JOB_COUNTER.add(1, &[]);

        let result = self
            .collection
            .find_one(doc! {Self::ID_FIELD: ObjectId::parse_str(job_id)?})
            .await?;

        if let Some(result) = result {
            Ok(result)
        } else {
            anyhow::bail!("Document not found");
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_jobs(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<database::model::PageSubset<domain::job::Job>> {
        tracing::debug!("Getting jobs");

        GET_JOBS_COUNTER.add(1, &[]);

        let skip = u64::from(page - 1) * u64::from(page_size);
        let filter = doc! {};

        let mut cursor = self
            .collection
            .find(filter.clone())
            .limit(i64::from(page_size))
            .skip(skip)
            .await?;

        let mut jobs = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            jobs.push(doc);
        }

        let total = self
            .collection
            .count_documents(filter)
            .await
            .map(usize::try_from)??;

        Ok(database::model::PageSubset::new(total, jobs))
    }
}
