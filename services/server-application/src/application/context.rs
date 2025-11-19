use crate::http::http_server::HttpServer;
use crate::messaging::consumer::MessageConsumer;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct ApplicationState {
    message_producer: OnceCell<MessageProducer>,
    message_consumer: OnceCell<MessageConsumer>,
    http_server: OnceCell<HttpServer>,
}

impl ApplicationState {
    pub fn message_producer(&self) -> &MessageProducer {
        self.message_producer
            .get()
            .expect("Message producer not initialized")
    }

    pub fn set_message_producer(&self, message_producer: MessageProducer) -> Result<()> {
        self.message_producer
            .set(message_producer)
            .map_err(|_| anyhow::anyhow!("Failed to set message producer in application state"))
    }

    pub fn message_consumer(&self) -> &MessageConsumer {
        self.message_consumer
            .get()
            .expect("Message consumer not initialized")
    }

    pub fn set_message_consumer(&self, message_consumer: MessageConsumer) -> Result<()> {
        self.message_consumer
            .set(message_consumer)
            .map_err(|_| anyhow::anyhow!("Failed to set message consumer in application state"))
    }

    pub fn http_server(&self) -> &HttpServer {
        self.http_server.get().expect("HTTP server not initialized")
    }

    pub fn set_http_server(&self, http_server: HttpServer) -> Result<()> {
        self.http_server
            .set(http_server)
            .map_err(|_| anyhow::anyhow!("Failed to set HTTP server in application state"))
    }
}

pub type SharedApplicationState = Arc<RwLock<ApplicationState>>;

pub async fn create_application_state() -> Result<SharedApplicationState> {
    let application_state = Arc::new(RwLock::new(ApplicationState::default()));

    let message_producer = MessageProducer::new()?;
    let message_consumer = MessageConsumer::new(application_state.clone())?;
    let http_server = HttpServer::new(8080);

    let application_state_guard = application_state.read().await;
    application_state_guard.set_message_producer(message_producer)?;
    application_state_guard.set_message_consumer(message_consumer)?;
    application_state_guard.set_http_server(http_server)?;
    drop(application_state_guard);

    Ok(application_state)
}

pub async fn start_application(application_state: SharedApplicationState) -> Result<()> {
    // Start the different components of the application
    let application_state_guard = application_state.read().await;
    let mut handles = [
        application_state_guard.http_server().start(),
        application_state_guard.message_consumer().start(),
    ];
    drop(application_state_guard);

    for handle in handles.iter_mut().flatten() {
        handle.await?;
    }

    Ok(())
}
