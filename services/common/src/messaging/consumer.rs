use crate::counter;
use crate::messaging::opentelemetry::KafkaHeaderContextExtractor;
use crate::messaging::opentelemetry::should_instrument_kafka;
use anyhow::Result;
use rdkafka::Message as _;
use rdkafka::consumer::Consumer as _;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::Instrument as _;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

counter!(
    MESSAGE_RECEIVED_COUNTER,
    "consumer_messages_received",
    "Number of messages received by the Kafka consumer"
);
counter!(
    MESSAGE_ERROR_COUNTER,
    "consumer_messages_error",
    "Number of messages that encountered an error by the Kafka consumer"
);

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

pub trait MessageHandler<T>: Send + Sync + 'static {
    fn handle(&self, message: T) -> impl Future<Output = Result<()>> + Send;
}

pub struct MessageConsumer<T, H> {
    consumers: Vec<Arc<KafkaConsumer>>,
    handler: Arc<H>,
    topic: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T, H> MessageConsumer<T, H>
where
    T: Send + 'static + for<'a> TryFrom<&'a str, Error = anyhow::Error>,
    H: MessageHandler<T>,
{
    const KAFKA_URI_ENV_VAR: &str = "KAFKA_URI";
    const DEFAULT_KAFKA_URI: &str = "127.0.0.1:9092";

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

    pub fn new(
        handler: Arc<H>,
        topic: &'static str,
        group_id: &'static str,
        concurrency: usize,
    ) -> Result<Self> {
        tracing::debug!("Initializing the Kafka consumer");

        let kafka_uri = std::env::var(Self::KAFKA_URI_ENV_VAR)
            .unwrap_or_else(|_| Self::DEFAULT_KAFKA_URI.to_string());
        let topics = vec![topic];

        let mut consumers = Vec::with_capacity(concurrency);
        for _ in 0..concurrency {
            let consumer = Arc::new(Self::create_consumer(&kafka_uri, group_id)?);
            consumer.subscribe(&topics)?;

            consumers.push(consumer);
        }

        Ok(Self {
            consumers,
            handler,
            topic,
            _marker: PhantomData,
        })
    }

    pub fn start(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<Result<()>>> {
        tracing::debug!("Start the Kafka consumer");

        self.consumers
            .iter()
            .map(|consumer| {
                let consumer_cloned = Arc::clone(consumer);
                let handler = Arc::clone(&self.handler);
                let topic = self.topic;
                let shutdown = shutdown.clone();

                tokio::spawn(async move {
                    Self::worker_consumer(consumer_cloned, handler, topic, shutdown).await;
                    Ok(())
                })
            })
            .collect()
    }

    fn create_consumer(uri: impl AsRef<str>, group_id: &'static str) -> Result<KafkaConsumer> {
        let consumer_config = Self::create_config(uri, group_id);

        consumer_config
            .create_with_context(KafkaConsumerContext)
            .map_err(|err| anyhow::anyhow!(format!("Failed to create Kafka consumer: {err}")))
    }

    fn create_config(uri: impl AsRef<str>, group_id: &'static str) -> rdkafka::ClientConfig {
        let mut consumer_config = rdkafka::ClientConfig::new();

        consumer_config.set(Self::KAFKA_CONFIG_BOOTSTRAP_SERVERS, uri.as_ref());
        consumer_config.set(Self::KAFKA_CONFIG_GROUP_ID, group_id);
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
        handler: Arc<H>,
        topic: &'static str,
        shutdown: CancellationToken,
    ) {
        loop {
            let received = tokio::select! {
                () = shutdown.cancelled() => return,
                result = consumer.recv() => result,
            };
            match received {
                Ok(message) => {
                    let span = tracing::info_span!("messaging.receive", topic = topic);
                    if should_instrument_kafka() {
                        _ = span.set_parent(opentelemetry::global::get_text_map_propagator(
                            |propagator| {
                                propagator
                                    .extract(&KafkaHeaderContextExtractor::new(message.headers()))
                            },
                        ));
                    }
                    async {
                        tracing::info!("Received Kafka message on topic {}", topic);

                        MESSAGE_RECEIVED_COUNTER
                            .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);

                        let payload = match message.payload_view::<str>() {
                            None => {
                                tracing::warn!("No message found");

                                MESSAGE_ERROR_COUNTER
                                    .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);

                                return;
                            }
                            Some(Ok(value)) => match T::try_from(value) {
                                Ok(deserialize_value) => deserialize_value,
                                Err(err) => {
                                    tracing::error!("Error while deserializing message: {err:?}");

                                    MESSAGE_ERROR_COUNTER
                                        .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);

                                    return;
                                }
                            },
                            Some(Err(err)) => {
                                tracing::error!("Error while converting message payload: {err:?}");

                                MESSAGE_ERROR_COUNTER
                                    .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);

                                return;
                            }
                        };

                        if let Err(err) = handler.handle(payload).await {
                            tracing::error!("Failed to handle message: {err}");

                            MESSAGE_ERROR_COUNTER
                                .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);
                        }

                        if let Err(err) = consumer.store_offset_from_message(&message) {
                            tracing::error!("Failed to store the offset from the message: {err}");

                            MESSAGE_ERROR_COUNTER
                                .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);
                        }
                    }
                    .instrument(span)
                    .await;
                }
                Err(err) => {
                    tracing::error!("Kafka error: {err}");

                    MESSAGE_ERROR_COUNTER.add(1, &[opentelemetry::KeyValue::new("topic", topic)]);
                }
            }
        }
    }
}
