use crate::counter;
use axum::body::Body;
use axum::extract::Request;
use axum::response::IntoResponse;

counter!(
    FALLBACK_COUNTER,
    "http_server_fallback_requests",
    "Number of fallback requests"
);

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
