use crate::domain;
use crate::messaging::model::OperationRequest;
use crate::messaging::producer::MessageProducer;
use anyhow::Result;
use common::messaging::consumer::MessageConsumer as CommonConsumer;
use common::messaging::consumer::MessageHandler;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const TOPIC_NAME: &str = "application.operation.request";
const GROUP_ID: &str = "operation-request-group";
const CONCURRENCY: usize = 10;

pub struct OperationRequestHandler {
    message_producer: Arc<MessageProducer>,
}

impl OperationRequestHandler {
    pub const fn new(message_producer: Arc<MessageProducer>) -> Self {
        Self { message_producer }
    }
}

impl MessageHandler<OperationRequest> for OperationRequestHandler {
    fn handle(&self, message: OperationRequest) -> impl Future<Output = Result<()>> + Send {
        let message_producer = Arc::clone(&self.message_producer);
        async move {
            let result = match evalexpr::eval(message.request()) {
                Ok(value) => value.to_string(),
                Err(err) => err.to_string(),
            };
            let operation = domain::operation::Operation::new(
                message.job_id(),
                message.operation_id(),
                message.request(),
                result,
            );

            message_producer.send_operation_result(operation);
            Ok(())
        }
    }
}

pub struct MessageConsumer(CommonConsumer<OperationRequest, OperationRequestHandler>);

impl MessageConsumer {
    pub fn new(message_producer: Arc<MessageProducer>) -> Result<Self> {
        let handler = Arc::new(OperationRequestHandler::new(message_producer));
        Ok(Self(CommonConsumer::new(
            handler,
            TOPIC_NAME,
            GROUP_ID,
            CONCURRENCY,
        )?))
    }

    pub fn start(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<Result<()>>> {
        self.0.start(shutdown)
    }
}
