use crate::database::job_repository::JobRepository;
use crate::database::operation_repository::OperationRepository;
use anyhow::Result;
use mongodb::Client;

pub struct DatabaseClient {
    job_repository: JobRepository,
    operation_repository: OperationRepository,
}

impl DatabaseClient {
    const MONGODB_URI_ENV_VAR: &str = "MONGODB_URI";
    const DEFAULT_MONGODB_URI: &str =
        "mongodb://user:password@127.0.0.1:27017/?authSource=application";

    pub const DATABASE_NAME: &'static str = "application";

    #[allow(clippy::similar_names)]
    pub async fn new() -> Result<Self> {
        let mongodb_uri = std::env::var(Self::MONGODB_URI_ENV_VAR)
            .unwrap_or_else(|_| Self::DEFAULT_MONGODB_URI.to_string());

        let client = Client::with_uri_str(mongodb_uri)
            .await
            .expect("Failed to parse MongoDB URI");
        client.warm_connection_pool().await;

        let job_repository = JobRepository::new(client.clone());
        let operation_repository = OperationRepository::new(client).await?;

        Ok(Self {
            job_repository,
            operation_repository,
        })
    }

    pub const fn job_repository(&self) -> &JobRepository {
        &self.job_repository
    }

    pub const fn operation_repository(&self) -> &OperationRepository {
        &self.operation_repository
    }
}
