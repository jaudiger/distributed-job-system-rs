use crate::application::APPLICATION_NAME;
use crate::application::context::SharedApplicationState;
use crate::domain;
use crate::http;
use crate::http::model::PageParams;
use crate::http::utils::ErrorResponse;
use anyhow::Result;
use axum::Json;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::LazyLock;
use tracing::Instrument as _;

static CREATE_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_create_job_requests")
        .with_description("Number of create job requests")
        .build()
});

static DELETE_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_delete_job_requests")
        .with_description("Number of delete job requests")
        .build()
});

static GET_JOB_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_get_job_requests")
        .with_description("Number of get job requests")
        .build()
});

static GET_JOBS_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> = LazyLock::new(|| {
    opentelemetry::global::meter(APPLICATION_NAME)
        .u64_counter("http_server_get_jobs_requests")
        .with_description("Number of get jobs requests")
        .build()
});

pub struct JobController;

impl JobController {
    #[tracing::instrument(skip(body, state))]
    pub async fn create_job_endpoint_handler(
        State(state): State<SharedApplicationState>,
        body: String,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Creating a new job");

        CREATE_JOB_COUNTER.add(1, &[]);

        let lines = body.lines().count();
        let new_job = domain::job::Job::new_job(lines);

        let job_id = state
            .read()
            .await
            .database_client()
            .job_repository()
            .insert_job(&new_job)
            .await?;

        let new_operations = body
            .lines()
            .map(|request| domain::operation::Operation::new_operation(&job_id, request))
            .collect();

        // Add the operations to the database
        if let Err(err) = state
            .read()
            .await
            .database_client()
            .operation_repository()
            .insert_operations(&new_operations)
            .await
        {
            tracing::error!("Failed to insert operations: {err}");

            let () = state
                .read()
                .await
                .database_client()
                .job_repository()
                .delete_job(&job_id)
                .await?;
        }

        // Send operation request message asynchronously
        let state_cloned = state.clone();
        let job_id_cloned = job_id.clone();
        let parent_span = tracing::Span::current();
        tokio::spawn(
            async move {
                const CHUNK_SIZE: u32 = 128;

                if let Err(err) = state_cloned
                    .read()
                    .await
                    .database_client()
                    .operation_repository()
                    .get_batch_operations(
                        &job_id_cloned,
                        CHUNK_SIZE,
                        |operation: domain::operation::Operation| {
                            let state_cloned = state.clone();
                            async move {
                                state_cloned
                                    .read()
                                    .await
                                    .message_producer()
                                    .send_operation_request(operation);
                            }
                        },
                    )
                    .await
                {
                    tracing::error!(
                        "Failed to get batch operations for job {job_id_cloned}: {err}",
                    );
                }
            }
            .instrument(parent_span),
        );

        Ok(Json(http::model::NewJobResponse::new(
            job_id,
            new_job.operations(),
        )))
    }

    #[tracing::instrument(skip(state))]
    pub async fn delete_job_endpoint_handler(
        Path(job_id): Path<String>,
        State(state): State<SharedApplicationState>,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Deleting job {}", job_id);

        DELETE_JOB_COUNTER.add(1, &[]);

        let () = state
            .read()
            .await
            .database_client()
            .job_repository()
            .delete_job(&job_id)
            .await?;

        // Delete the operations asynchronously, since it can take time and we don't want to wait for this to finish before returning a response to the client
        let state_cloned = state.clone();
        let parent_span = tracing::Span::current();
        tokio::spawn(
            async move {
                if let Err(err) = state_cloned
                    .read()
                    .await
                    .database_client()
                    .operation_repository()
                    .delete_operations(&job_id)
                    .await
                {
                    tracing::error!("Failed to delete operations for job {job_id}: {err}");
                }
            }
            .instrument(parent_span),
        );

        Ok(Body::empty())
    }

    #[tracing::instrument(skip(state))]
    pub async fn get_job_endpoint_handler(
        Path(job_id): Path<String>,
        State(state): State<SharedApplicationState>,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Getting job {}", job_id);

        GET_JOB_COUNTER.add(1, &[]);

        let job = state
            .read()
            .await
            .database_client()
            .job_repository()
            .get_job(&job_id)
            .await?;

        let total_completed_operations = state
            .read()
            .await
            .database_client()
            .operation_repository()
            .get_total_completed_operations(job_id)
            .await?;

        Ok(Json(http::model::JobResponse::new(
            &job,
            total_completed_operations,
        )))
    }

    #[tracing::instrument(skip(state))]
    pub async fn get_jobs_endpoint_handler(
        Query(params): Query<PageParams>,
        State(state): State<SharedApplicationState>,
    ) -> Result<impl IntoResponse, ErrorResponse> {
        tracing::info!("Getting all the jobs");

        GET_JOBS_COUNTER.add(1, &[]);

        let page = params.page();
        let page_size = params.size();

        let jobs = state
            .read()
            .await
            .database_client()
            .job_repository()
            .get_jobs(page, page_size)
            .await?;

        Ok(Json(http::model::PageResponse::new(
            page,
            page_size,
            jobs.total(),
            jobs.items_subset()
                .iter()
                .map(http::model::MinimalJobResponse::from)
                .collect(),
        )))
    }
}
