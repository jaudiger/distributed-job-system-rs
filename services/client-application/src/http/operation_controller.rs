use crate::application::APPLICATION_NAME;
use crate::application::context::SharedApplicationState;
use crate::http;
use crate::http::model::PageParams;
use crate::http::utils::ErrorResponse;
use anyhow::Result;
use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::LazyLock;

static GET_OPERATION_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> =
    LazyLock::new(|| {
        opentelemetry::global::meter(APPLICATION_NAME)
            .u64_counter("http_server_get_operation_requests")
            .with_description("Number of get operation requests")
            .build()
    });

static GET_OPERATIONS_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> =
    LazyLock::new(|| {
        opentelemetry::global::meter(APPLICATION_NAME)
            .u64_counter("http_server_get_operations_requests")
            .with_description("Number of get operations requests")
            .build()
    });

pub struct OperationController;

impl OperationController {
    #[tracing::instrument(skip(state))]
    pub async fn get_operation_endpoint_handler(
        Path((job_id, operation_id)): Path<(String, String)>,
        State(state): State<SharedApplicationState>,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Getting operation {} for job {}", operation_id, job_id);

        GET_OPERATION_COUNTER.add(1, &[]);

        let operation = state
            .read()
            .await
            .database_client()
            .operation_repository()
            .get_operation(job_id, operation_id)
            .await?;

        Ok(Json(http::model::OperationResponse::from(operation)))
    }

    #[tracing::instrument(skip(state))]
    pub async fn get_operations_endpoint_handler(
        Path(job_id): Path<String>,
        Query(params): Query<PageParams>,
        State(state): State<SharedApplicationState>,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Getting all the operations job {}", job_id);

        GET_OPERATIONS_COUNTER.add(1, &[]);

        let page = params.page();
        let page_size = params.size();

        let operations = state
            .read()
            .await
            .database_client()
            .operation_repository()
            .get_operations(job_id, page, page_size)
            .await?;

        Ok(Json(http::model::PageResponse::new(
            page,
            page_size,
            operations.total(),
            operations
                .items_subset()
                .iter()
                .map(http::model::MinimalOperationResponse::from)
                .collect(),
        )))
    }
}
