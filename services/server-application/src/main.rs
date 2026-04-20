use crate::application::APPLICATION_NAME;
use crate::application::context::create_application;
use crate::application::context::start_application;
use anyhow::Result;
use common::application::opentelemetry::OpentelemetryHandler;

mod application;
mod domain;
mod messaging;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the OpenTelemetry stack
    let opentelemetry_handler = OpentelemetryHandler::new(APPLICATION_NAME)?;

    // Start the application
    let result = async {
        let application = create_application().await?;
        start_application(application).await
    }
    .await;

    // Shutdown the OpenTelemetry stack
    opentelemetry_handler.shutdown();

    result
}
