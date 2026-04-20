use crate::database::database_client::DatabaseClient;
use crate::http::JobController;
use crate::http::OperationController;
use crate::messaging::consumer::MessageConsumer;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::handler::Handler as _;
use axum::routing::get;
use common::http::HttpServer;
use futures::future::try_join_all;
use std::sync::Arc;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio_util::sync::CancellationToken;

const HTTP_PORT: u16 = 8080;
const BODY_LIMIT: DefaultBodyLimit = DefaultBodyLimit::max(10 * 1024 * 1024); // 10MB

pub struct ApplicationState {
    database_client: Arc<DatabaseClient>,
    message_producer: Arc<MessageProducer>,
}

impl ApplicationState {
    pub fn database_client(&self) -> &DatabaseClient {
        &self.database_client
    }

    pub fn message_producer(&self) -> &MessageProducer {
        &self.message_producer
    }
}

pub type SharedApplicationState = Arc<ApplicationState>;

pub struct Application {
    consumer: MessageConsumer,
    http_server: HttpServer,
}

pub async fn create_application() -> Result<Application> {
    let database_client = Arc::new(DatabaseClient::new().await?);
    let message_producer = Arc::new(MessageProducer::new()?);
    let consumer = MessageConsumer::new(Arc::clone(&database_client))?;

    let application_state = Arc::new(ApplicationState {
        database_client,
        message_producer,
    });

    let router = build_router(Arc::clone(&application_state));
    let http_server = HttpServer::new(HTTP_PORT, router);

    Ok(Application {
        consumer,
        http_server,
    })
}

fn build_router(application_state: SharedApplicationState) -> Router {
    Router::new()
        .route(
            "/api/jobs",
            get(JobController::get_jobs_endpoint_handler)
                .post(JobController::create_job_endpoint_handler.layer(BODY_LIMIT)),
        )
        .route(
            "/api/jobs/{job_id}",
            get(JobController::get_job_endpoint_handler)
                .delete(JobController::delete_job_endpoint_handler),
        )
        .route(
            "/api/jobs/{job_id}/operations",
            get(OperationController::get_operations_endpoint_handler),
        )
        .route(
            "/api/jobs/{job_id}/operations/{operation_id}",
            get(OperationController::get_operation_endpoint_handler),
        )
        .with_state(application_state)
}

pub async fn start_application(application: Application) -> Result<()> {
    let shutdown = CancellationToken::new();
    let Application {
        consumer,
        http_server,
    } = application;

    let handles = http_server
        .start(&shutdown)
        .into_iter()
        .chain(consumer.start(&shutdown));

    let services = try_join_all(handles.map(|handle| async move { handle.await? }));
    let signal = wait_for_shutdown_signal(shutdown.clone());
    tokio::pin!(services, signal);

    tokio::select! {
        res = &mut services => {
            shutdown.cancel();
            first_error(res.map(|_| ()), signal.await)
        }
        res = &mut signal => first_error(res, services.await.map(|_| ())),
    }
}

fn first_error(primary: Result<()>, secondary: Result<()>) -> Result<()> {
    if let (Err(_), Err(err)) = (&primary, &secondary) {
        tracing::error!("Additional shutdown error: {err}");
    }
    primary.and(secondary)
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
