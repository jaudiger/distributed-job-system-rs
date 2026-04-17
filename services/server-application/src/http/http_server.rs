use crate::http::fallback_controller::FallbackController;
use crate::http::health_check::HealthCheckController;
use anyhow::Result;
use axum::Router;
use axum::routing::get;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;

#[derive(Default)]
pub struct HttpServer {
    port: u16,
}

impl HttpServer {
    const DEFAULT_LISTENER_ADDR: [u8; 4] = [0, 0, 0, 0];

    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn start(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<Result<()>>> {
        let port = self.port;
        let shutdown = shutdown.clone();

        vec![tokio::spawn(async move {
            Self::worker_axum(port, shutdown).await
        })]
    }

    async fn worker_axum(port: u16, shutdown: CancellationToken) -> Result<()> {
        let trace_layer =
            TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true));

        // Construct the routes
        let router = Router::new()
            .route(
                "/health",
                get(HealthCheckController::get_status_endpoint_handler),
            )
            .fallback(FallbackController::fallback_endpoint_handler)
            .layer(trace_layer);

        let addr = SocketAddr::from((Self::DEFAULT_LISTENER_ADDR, port));
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Starting HTTP Server on {}", listener.local_addr()?);

        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown.cancelled().await })
            .await?;

        Ok(())
    }
}
