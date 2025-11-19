use crate::application::APPLICATION_NAME;
use crate::http::model::HealthCheckResponse;
use axum::Json;
use axum::response::IntoResponse;
use std::sync::LazyLock;

static HEALTH_CHECK_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_health_check_requests")
        .with_description("Number of health check requests")
        .build()
});

pub struct HealthCheckController;

impl HealthCheckController {
    #[allow(clippy::unused_async)]
    #[tracing::instrument(level = "debug")]
    pub async fn get_status_endpoint_handler() -> impl IntoResponse {
        tracing::debug!("Getting service status");

        HEALTH_CHECK_COUNTER.add(1, &[]);

        Json(HealthCheckResponse::up())
    }
}
