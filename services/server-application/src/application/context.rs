use crate::messaging::consumer::MessageConsumer;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use axum::Router;
use common::http::HttpServer;
use std::sync::Arc;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const HTTP_PORT: u16 = 8080;

pub struct Application {
    consumer: MessageConsumer,
    http_server: HttpServer,
}

pub async fn create_application() -> Result<Application> {
    let message_producer = Arc::new(MessageProducer::new()?);
    let consumer = MessageConsumer::new(message_producer)?;
    let http_server = HttpServer::new(HTTP_PORT, Router::new());

    Ok(Application {
        consumer,
        http_server,
    })
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

    let mut tasks = JoinSet::new();
    for handle in handles {
        tasks.spawn(async move { handle.await? });
    }

    tasks.spawn(wait_for_shutdown_signal(shutdown.clone()));

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
