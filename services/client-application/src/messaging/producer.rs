use crate::domain;
use crate::messaging::model::OperationRequest;
use anyhow::Result;
use common::messaging::producer::MessageProducer as CommonProducer;

pub struct MessageProducer(CommonProducer<OperationRequest>);

impl MessageProducer {
    const TOPIC_NAME: &'static str = "application.operation.request";

    pub fn new() -> Result<Self> {
        Ok(Self(CommonProducer::new(Self::TOPIC_NAME)?))
    }

    pub fn send_operation_request(&self, operation: domain::operation::Operation) {
        self.0.send(OperationRequest::from(operation));
    }
}
