use crate::application::APPLICATION_NAME;
use crate::application::context::SharedApplicationState;
use crate::domain;
use crate::messaging::opentelemetry::KafkaHeaderContextExtractor;
use crate::messaging::opentelemetry::should_instrument_kafka;
use anyhow::Result;
use rdkafka::Message as _;
use rdkafka::consumer::Consumer as _;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::task::JoinHandle;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

static MESSAGE_RECEIVED_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> =
    LazyLock::new(|| {
        opentelemetry::global::meter(APPLICATION_NAME)
            .u64_counter("consumer_messages_received")
            .with_description("Number of messages received by the Kafka consumer")
            .build()
    });
static MESSAGE_ERROR_COUNTER: LazyLock<opentelemetry::metrics::Counter<u64>> =
    LazyLock::new(|| {
        opentelemetry::global::meter(APPLICATION_NAME)
            .u64_counter("consumer_messages_error")
            .with_description("Number of messages that encountered an error by the Kafka consumer")
            .build()
    });

#[derive(Default)]
pub struct KafkaConsumerContext;

impl rdkafka::ClientContext for KafkaConsumerContext {}

impl rdkafka::consumer::ConsumerContext for KafkaConsumerContext {
    fn commit_callback(
        &self,
        result: rdkafka::error::KafkaResult<()>,
        topic_partitions: &rdkafka::TopicPartitionList,
    ) {
        match result {
            Ok(()) => {
                tracing::debug!("Offsets committed successfully");
            }
            Err(err) => {
                let topics = topic_partitions
                    .elements()
                    .iter()
                    .map(rdkafka::topic_partition_list::TopicPartitionListElem::topic)
                    .collect::<Vec<&str>>()
                    .join(", ");

                tracing::error!("Error on topic(s) '{topics}' while committing offsets: {err}");
            }
        }
    }
}

pub type KafkaConsumer = rdkafka::consumer::StreamConsumer<KafkaConsumerContext>;

pub struct MessageConsumer {
    consumers: Vec<Arc<KafkaConsumer>>,
    application_state: SharedApplicationState,
}

impl MessageConsumer {
    const KAFKA_URI_ENV_VAR: &str = "KAFKA_URI";
    const DEFAULT_KAFKA_URI: &str = "127.0.0.1:9092";

    const CONCURRENCY: usize = 10;
    const GROUP_ID: &str = "operation-request-group";
    const TOPIC_NAME: &str = "application.operation.request";

    const KAFKA_CONFIG_AUTO_OFFSET_RESET: &str = "auto.offset.reset";
    const KAFKA_CONFIG_BOOTSTRAP_SERVERS: &str = "bootstrap.servers";
    const KAFKA_CONFIG_ENABLE_AUTO_COMMIT: &str = "enable.auto.commit";
    const KAFKA_CONFIG_ENABLE_AUTO_OFFSET_STORE: &str = "enable.auto.offset.store";
    const KAFKA_CONFIG_GROUP_ID: &str = "group.id";
    const KAFKA_CONFIG_QUEUED_MAX_MESSAGES_KBYTES: &str = "queued.max.messages.kbytes";
    const KAFKA_CONFIG_QUEUED_MIN_MESSAGES: &str = "queued.min.messages";
    const KAFKA_CONFIG_RECONNECT_BACKOFF_MAX_MS: &str = "reconnect.backoff.max.ms";
    const KAFKA_CONFIG_RECONNECT_BACKOFF_MS: &str = "reconnect.backoff.ms";

    const KAFKA_CONFIG_AUTO_OFFSET_RESET_DEFAULT_VALUE: &str = "earliest";
    const KAFKA_CONFIG_AUTO_COMMIT_DEFAULT_VALUE: &str = "true";
    const KAFKA_CONFIG_AUTO_OFFSET_STORE_DEFAULT_VALUE: &str = "false";
    const KAFKA_CONFIG_QUEUED_MAX_MESSAGES_KBYTES_DEFAULT_VALUE: &str = "65536";
    const KAFKA_CONFIG_QUEUED_MIN_MESSAGES_DEFAULT_VALUE: &str = "1024";
    const KAFKA_CONFIG_RECONNECT_BACKOFF_MAX_MS_DEFAULT_VALUE: &str = "15000";
    const KAFKA_CONFIG_RECONNECT_BACKOFF_MS_DEFAULT_VALUE: &str = "5000";

    pub fn new(application_state: SharedApplicationState) -> Result<Self> {
        tracing::debug!("Initializing the Kafka consumer");

        let kafka_uri = std::env::var(Self::KAFKA_URI_ENV_VAR)
            .unwrap_or_else(|_| Self::DEFAULT_KAFKA_URI.to_string());
        let topics = vec![Self::TOPIC_NAME];

        let mut consumers = Vec::with_capacity(Self::CONCURRENCY);
        for _ in 0..Self::CONCURRENCY {
            let consumer = Arc::new(Self::create_consumer(&kafka_uri)?);
            consumer.subscribe(&topics)?;

            consumers.push(consumer);
        }

        Ok(Self {
            consumers,
            application_state,
        })
    }

    pub fn start(&self) -> Vec<JoinHandle<()>> {
        tracing::debug!("Start the Kafka consumer");

        self.consumers
            .iter()
            .map(|consumer| {
                let consumer_cloned = Arc::clone(consumer);
                let application_state = Arc::clone(&self.application_state);

                tokio::spawn(async move {
                    Self::worker_consumer(consumer_cloned, application_state).await;
                })
            })
            .collect()
    }

    fn create_consumer(uri: impl AsRef<str>) -> Result<KafkaConsumer> {
        let consumer_config = Self::create_config(uri);

        // Create Kafka consumer
        consumer_config
            .create_with_context(KafkaConsumerContext)
            .map_err(|err| anyhow::anyhow!(format!("Failed to create Kafka consumer: {err}")))
    }

    fn create_config(uri: impl AsRef<str>) -> rdkafka::ClientConfig {
        let mut consumer_config = rdkafka::ClientConfig::new();

        // Default Kafka consumer configuration
        consumer_config.set(Self::KAFKA_CONFIG_BOOTSTRAP_SERVERS, uri.as_ref());
        consumer_config.set(Self::KAFKA_CONFIG_GROUP_ID, Self::GROUP_ID);
        consumer_config.set(
            Self::KAFKA_CONFIG_AUTO_OFFSET_RESET,
            Self::KAFKA_CONFIG_AUTO_OFFSET_RESET_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_ENABLE_AUTO_COMMIT,
            Self::KAFKA_CONFIG_AUTO_COMMIT_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_ENABLE_AUTO_OFFSET_STORE,
            Self::KAFKA_CONFIG_AUTO_OFFSET_STORE_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_QUEUED_MIN_MESSAGES,
            Self::KAFKA_CONFIG_QUEUED_MIN_MESSAGES_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_QUEUED_MAX_MESSAGES_KBYTES,
            Self::KAFKA_CONFIG_QUEUED_MAX_MESSAGES_KBYTES_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_RECONNECT_BACKOFF_MS,
            Self::KAFKA_CONFIG_RECONNECT_BACKOFF_MS_DEFAULT_VALUE,
        );
        consumer_config.set(
            Self::KAFKA_CONFIG_RECONNECT_BACKOFF_MAX_MS,
            Self::KAFKA_CONFIG_RECONNECT_BACKOFF_MAX_MS_DEFAULT_VALUE,
        );
        consumer_config.set_log_level(rdkafka::config::RDKafkaLogLevel::Info);

        consumer_config
    }

    async fn worker_consumer(
        consumer: Arc<KafkaConsumer>,
        application_state: SharedApplicationState,
    ) {
        loop {
            match consumer.recv().await {
                Ok(message) => {
                    let span = tracing::info_span!("messaging.receive", topic = Self::TOPIC_NAME);
                    if should_instrument_kafka() {
                        let _ = span.set_parent(opentelemetry::global::get_text_map_propagator(
                            |propagator| {
                                propagator
                                    .extract(&KafkaHeaderContextExtractor::new(message.headers()))
                            },
                        ));
                    }
                    let _enter = span.enter();

                    tracing::info!("Received Kafka message on topic {}", Self::TOPIC_NAME);

                    MESSAGE_RECEIVED_COUNTER.add(
                        1,
                        &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                    );

                    let operation_request = match message.payload_view::<str>() {
                        None => {
                            tracing::warn!("No message found");

                            MESSAGE_ERROR_COUNTER.add(
                                1,
                                &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                            );

                            continue;
                        }
                        Some(Ok(value)) => match super::model::OperationRequest::try_from(value) {
                            Ok(deserialize_value) => deserialize_value,
                            Err(err) => {
                                tracing::error!("Error while deserializing message: {err:?}");

                                MESSAGE_ERROR_COUNTER.add(
                                    1,
                                    &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                                );

                                continue;
                            }
                        },
                        Some(Err(err)) => {
                            tracing::error!("Error while converting message payload: {err:?}");

                            MESSAGE_ERROR_COUNTER.add(
                                1,
                                &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                            );

                            continue;
                        }
                    };

                    let result = match evalexpr::eval(operation_request.request()) {
                        Ok(value) => value.to_string(),
                        Err(err) => err.to_string(),
                    };
                    let operation = domain::operation::Operation::new(
                        operation_request.job_id(),
                        operation_request.operation_id(),
                        operation_request.request(),
                        result,
                    );

                    application_state
                        .read()
                        .await
                        .message_producer()
                        .send_operation_result(operation);

                    if let Err(err) = consumer.store_offset_from_message(&message) {
                        tracing::error!("Failed to store the offset from the message: {err}");

                        MESSAGE_ERROR_COUNTER.add(
                            1,
                            &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                        );
                    }
                }
                Err(err) => {
                    tracing::error!("Kafka error: {err}");

                    MESSAGE_ERROR_COUNTER.add(
                        1,
                        &[opentelemetry::KeyValue::new("topic", Self::TOPIC_NAME)],
                    );
                }
            }
        }
    }
}
