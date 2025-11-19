use crate::application::APPLICATION_NAME;
use axum::body::Body;
use axum::extract::Request;
use axum::response::IntoResponse;
use std::sync::LazyLock;

static FALLBACK_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_fallback_requests")
        .with_description("Number of fallback requests")
        .build()
});

pub struct FallbackController;

impl FallbackController {
    #[allow(clippy::unused_async)]
    #[tracing::instrument]
    pub async fn fallback_endpoint_handler(req: Request<Body>) -> impl IntoResponse {
        tracing::warn!(
            "Unexpected route targeted: {} {}",
            req.method(),
            req.uri().path()
        );

        FALLBACK_COUNTER.add(1, &[]);

        (axum::http::StatusCode::NOT_FOUND, "Unexpected route")
    }
}
