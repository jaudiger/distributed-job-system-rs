use crate::messaging::consumer::MessageConsumer;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use axum::Router;
use common::http::HttpServer;
use futures::future::try_join_all;
use std::sync::Arc;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
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
