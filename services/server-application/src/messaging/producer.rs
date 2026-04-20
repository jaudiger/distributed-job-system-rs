use crate::domain;
use crate::messaging::model::OperationResult;
use anyhow::Result;
use common::messaging::producer::MessageProducer as CommonProducer;

pub struct MessageProducer(CommonProducer<OperationResult>);

impl MessageProducer {
    const TOPIC_NAME: &'static str = "application.operation.response";

    pub fn new() -> Result<Self> {
        Ok(Self(CommonProducer::new(Self::TOPIC_NAME)?))
    }

    pub fn send_operation_result(&self, operation: domain::operation::Operation) {
        self.0.send(OperationResult::from(operation));
    }
}
