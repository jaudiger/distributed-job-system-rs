use crate::application::counter;
use crate::http::model::HealthCheckResponse;
use axum::Json;
use axum::response::IntoResponse;

counter!(
    HEALTH_CHECK_COUNTER,
    "http_server_health_check_requests",
    "Number of health check requests"
);

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
