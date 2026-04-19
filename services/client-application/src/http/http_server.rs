use crate::application::context::SharedApplicationState;
use crate::http::fallback_controller::FallbackController;
use crate::http::health_check_controller::HealthCheckController;
use crate::http::job_controller::JobController;
use crate::http::operation_controller::OperationController;
use anyhow::Result;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::handler::Handler;
use axum::routing::get;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;

pub struct HttpServer {
    port: u16,
}

impl HttpServer {
    const DEFAULT_LISTENER_ADDR: [u8; 4] = [0, 0, 0, 0];
    const BODY_LIMIT: DefaultBodyLimit = DefaultBodyLimit::max(10 * 1024 * 1024); // 10MB

    pub fn new(port: u16) -> Self {
        tracing::debug!("Initializing the HTTP server");

        Self { port }
    }

    pub fn start(
        &self,
        application_state: SharedApplicationState,
        shutdown: &CancellationToken,
    ) -> Vec<JoinHandle<Result<()>>> {
        tracing::info!("Starting the HTTP server on port {}", self.port);

        let port = self.port;
        let shutdown = shutdown.clone();

        vec![tokio::spawn(async move {
            Self::worker_axum(port, application_state, shutdown).await
        })]
    }

    async fn worker_axum(
        port: u16,
        application_state: SharedApplicationState,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let trace_layer =
            TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true));

        // Construct the routes
        let router = Router::new()
            .route(
                "/health",
                get(HealthCheckController::get_status_endpoint_handler),
            )
            .route(
                "/api/jobs",
                get(JobController::get_jobs_endpoint_handler)
                    .post(JobController::create_job_endpoint_handler.layer(Self::BODY_LIMIT)),
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
            .fallback(FallbackController::fallback_endpoint_handler)
            .layer(trace_layer)
            .with_state(application_state);

        let addr = SocketAddr::from((Self::DEFAULT_LISTENER_ADDR, port));
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Starting HTTP Server on {}", listener.local_addr()?);

        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown.cancelled().await })
            .await?;

        Ok(())
    }
}
