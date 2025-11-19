use crate::application::APPLICATION_NAME;
use crate::database;
use crate::database::database_client::DatabaseClient;
use crate::domain;
use anyhow::Result;
use futures::TryStreamExt;
use mongodb::Client;
use mongodb::Collection;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use std::sync::LazyLock;

static INSERT_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("database_insert_job_requests")
        .with_description("Number of insert job requests")
        .build()
});

static DELETE_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("database_delete_job_requests")
        .with_description("Number of delete job requests")
        .build()
});

static GET_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("database_get_job_requests")
        .with_description("Number of get job requests")
        .build()
});

static GET_JOBS_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("database_get_jobs_requests")
        .with_description("Number of get jobs requests")
        .build()
});

pub struct JobRepository {
    client: Client,
}

impl JobRepository {
    const COLLECTION_NAME: &'static str = "job";

    const ID_FIELD: &'static str = "_id";

    pub fn new(client: Client) -> Self {
        tracing::debug!("Initializing the MongoDB job repository");

        Self { client }
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_job(&self, job: &domain::job::Job) -> Result<String> {
        tracing::debug!("Inserting a job");

        INSERT_JOB_COUNTER.add(1, &[]);

        let job_collection: Collection<domain::job::Job> = self
            .client
            .database(DatabaseClient::DATABASE_NAME)
            .collection(Self::COLLECTION_NAME);

        let result = job_collection.insert_one(job).await?;

        Ok(result
            .inserted_id
            .as_object_id()
            .expect("No ObjectId returned")
            .to_string())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_job(&self, job_id: impl AsRef<str> + std::fmt::Debug) -> Result<()> {
        tracing::debug!("Deleting job with id {}", job_id.as_ref());

        DELETE_JOB_COUNTER.add(1, &[]);

        let job_collection: Collection<domain::job::Job> = self
            .client
            .database(DatabaseClient::DATABASE_NAME)
            .collection(Self::COLLECTION_NAME);

        let result = job_collection
            .delete_one(doc! {Self::ID_FIELD: ObjectId::parse_str(job_id)?})
            .await?;

        if result.deleted_count == 0 {
            anyhow::bail!("Document not found");
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_job(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
    ) -> Result<domain::job::Job> {
        tracing::debug!("Getting job with id: {}", job_id.as_ref());

        GET_JOB_COUNTER.add(1, &[]);

        let job_collection: Collection<domain::job::Job> = self
            .client
            .database(DatabaseClient::DATABASE_NAME)
            .collection(Self::COLLECTION_NAME);

        let result = job_collection
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
        page: usize,
        page_size: usize,
    ) -> Result<database::model::PageSubset<domain::job::Job>> {
        tracing::debug!("Getting jobs");

        GET_JOBS_COUNTER.add(1, &[]);

        let job_collection: Collection<domain::job::Job> = self
            .client
            .database(DatabaseClient::DATABASE_NAME)
            .collection(Self::COLLECTION_NAME);

        let skip = ((page - 1) * page_size) as u64;

        #[allow(clippy::cast_possible_wrap)]
        let mut cursor = job_collection
            .find(doc! {})
            .limit(page_size as i64)
            .skip(skip)
            .await?;

        let mut jobs = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            jobs.push(doc);
        }

        let total = job_collection
            .estimated_document_count()
            .await
            .map(usize::try_from)??;

        Ok(database::model::PageSubset::new(total, jobs))
    }
}
