use crate::application::context::create_application_state;
use crate::application::context::start_application;
use crate::application::opentelemetry::OpentelemetryHandler;
use anyhow::Result;
mod application;
mod database;
mod domain;
mod http;
mod messaging;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the OpenTelemetry stack
    let _opentelemetry_handler = OpentelemetryHandler::new()?;

    // Start the application
    let application_state = create_application_state().await?;
    start_application(application_state).await?;

    Ok(())
}
