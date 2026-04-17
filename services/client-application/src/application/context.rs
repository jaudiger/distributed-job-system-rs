use crate::database::database_client::DatabaseClient;
use crate::http::http_server::HttpServer;
use crate::messaging::consumer::MessageConsumer;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use std::sync::Arc;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct ApplicationState {
    database_client: OnceCell<DatabaseClient>,
    message_producer: OnceCell<MessageProducer>,
    message_consumer: OnceCell<MessageConsumer>,
    http_server: OnceCell<HttpServer>,
}

impl ApplicationState {
    pub fn database_client(&self) -> &DatabaseClient {
        self.database_client
            .get()
            .expect("Database client not initialized")
    }

    pub fn set_database_client(&self, database_client: DatabaseClient) -> Result<()> {
        self.database_client
            .set(database_client)
            .map_err(|_| anyhow::anyhow!("Failed to set database client in application state"))
    }

    pub fn message_producer(&self) -> &MessageProducer {
        self.message_producer
            .get()
            .expect("Message producer not initialized")
    }

    pub fn set_message_producer(&self, message_producer: MessageProducer) -> Result<()> {
        self.message_producer
            .set(message_producer)
            .map_err(|_| anyhow::anyhow!("Failed to set message producer in application state"))
    }

    pub fn message_consumer(&self) -> &MessageConsumer {
        self.message_consumer
            .get()
            .expect("Message consumer not initialized")
    }

    pub fn set_message_consumer(&self, message_consumer: MessageConsumer) -> Result<()> {
        self.message_consumer
            .set(message_consumer)
            .map_err(|_| anyhow::anyhow!("Failed to set message consumer in application state"))
    }

    pub fn http_server(&self) -> &HttpServer {
        self.http_server.get().expect("HTTP server not initialized")
    }

    pub fn set_http_server(&self, http_server: HttpServer) -> Result<()> {
        self.http_server
            .set(http_server)
            .map_err(|_| anyhow::anyhow!("Failed to set HTTP server in application state"))
    }
}

pub type SharedApplicationState = Arc<RwLock<ApplicationState>>;

pub async fn create_application_state() -> Result<SharedApplicationState> {
    let application_state = Arc::new(RwLock::new(ApplicationState::default()));

    let database_client = DatabaseClient::new().await?;
    let message_producer = MessageProducer::new()?;
    let message_consumer = MessageConsumer::new(application_state.clone())?;
    let http_server = HttpServer::new(8080, application_state.clone());

    let application_state_guard = application_state.read().await;
    application_state_guard.set_database_client(database_client)?;
    application_state_guard.set_message_producer(message_producer)?;
    application_state_guard.set_message_consumer(message_consumer)?;
    application_state_guard.set_http_server(http_server)?;
    drop(application_state_guard);

    Ok(application_state)
}

pub async fn start_application(application_state: SharedApplicationState) -> Result<()> {
    let shutdown = CancellationToken::new();

    // Start the different components of the application
    let application_state_guard = application_state.read().await;
    let handles = application_state_guard
        .http_server()
        .start(&shutdown)
        .into_iter()
        .chain(application_state_guard.message_consumer().start(&shutdown));
    drop(application_state_guard);

    let mut tasks = JoinSet::new();
    for handle in handles {
        tasks.spawn(async move { handle.await? });
    }

    tasks.spawn(wait_for_shutdown_signal(shutdown.clone()));

    // The first task to finish signals everyone else to drain.
    let first = tasks.join_next().await;
    shutdown.cancel();

    let mut outcome = first.map_or_else(|| Ok(()), flatten_join_outcome);

    while let Some(result) = tasks.join_next().await {
        let task_outcome = flatten_join_outcome(result);
        if outcome.is_ok() {
            outcome = task_outcome;
        } else if let Err(err) = task_outcome {
            tracing::error!("Additional shutdown error: {err}");
        }
    }

    outcome
}

fn flatten_join_outcome(
    result: std::result::Result<Result<()>, tokio::task::JoinError>,
) -> Result<()> {
    result.map_err(anyhow::Error::from).and_then(|r| r)
}

async fn wait_for_shutdown_signal(shutdown: CancellationToken) -> Result<()> {
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => tracing::info!("SIGTERM received, starting graceful shutdown"),
        _ = sigint.recv() => tracing::info!("SIGINT received, starting graceful shutdown"),
        () = shutdown.cancelled() => {}
    }

    shutdown.cancel();
    Ok(())
}
