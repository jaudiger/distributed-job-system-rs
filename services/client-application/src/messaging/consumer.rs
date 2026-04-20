use crate::database::database_client::DatabaseClient;
use crate::messaging::model::OperationResult;
use anyhow::Result;
use common::messaging::consumer::MessageConsumer as CommonConsumer;
use common::messaging::consumer::MessageHandler;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const TOPIC_NAME: &str = "application.operation.response";
const GROUP_ID: &str = "operation-response-group";
const CONCURRENCY: usize = 20;

pub struct OperationResultHandler {
    database_client: Arc<DatabaseClient>,
}

impl OperationResultHandler {
    pub const fn new(database_client: Arc<DatabaseClient>) -> Self {
        Self { database_client }
    }
}

impl MessageHandler<OperationResult> for OperationResultHandler {
    fn handle(&self, message: OperationResult) -> impl Future<Output = Result<()>> + Send {
        let database_client = Arc::clone(&self.database_client);
        async move {
            database_client
                .operation_repository()
                .update_operation(message.job_id(), message.operation_id(), message.result())
                .await
        }
    }
}

pub struct MessageConsumer(CommonConsumer<OperationResult, OperationResultHandler>);

impl MessageConsumer {
    pub fn new(database_client: Arc<DatabaseClient>) -> Result<Self> {
        let handler = Arc::new(OperationResultHandler::new(database_client));
        Ok(Self(CommonConsumer::new(
            handler,
            TOPIC_NAME,
            GROUP_ID,
            CONCURRENCY,
        )?))
    }

    pub fn start(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<Result<()>>> {
        self.0.start(shutdown)
    }
}
