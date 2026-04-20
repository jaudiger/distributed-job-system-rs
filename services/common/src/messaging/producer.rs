use crate::counter;
use crate::messaging::opentelemetry::KafkaHeaderContextInjector;
use crate::messaging::opentelemetry::should_instrument_kafka;
use anyhow::Result;
use std::marker::PhantomData;
use tracing::Instrument as _;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

counter!(
    MESSAGE_SENT_COUNTER,
    "producer_messages_sent",
    "Number of messages sent by the Kafka producer"
);
counter!(
    MESSAGE_ERROR_COUNTER,
    "producer_messages_error",
    "Number of messages that encountered an error by the Kafka producer"
);

#[derive(Default)]
struct KafkaProducerContext;

impl rdkafka::ClientContext for KafkaProducerContext {}

impl rdkafka::producer::ProducerContext for KafkaProducerContext {
    type DeliveryOpaque = ();

    fn delivery(
        &self,
        delivery_result: &rdkafka::producer::DeliveryResult<'_>,
        _delivery_opaque: Self::DeliveryOpaque,
    ) {
        match delivery_result {
            Ok(_) => {
                tracing::debug!("Message delivered successfully");
            }
            Err((kafka_error, _borrowed_message)) => {
                tracing::error!("Error while delivering message: {kafka_error}");
            }
        }
    }
}

type KafkaProducer = rdkafka::producer::FutureProducer<KafkaProducerContext>;

pub struct MessageProducer<T> {
    producer: KafkaProducer,
    topic: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> MessageProducer<T>
where
    T: serde::Serialize + Send + 'static,
{
    const KAFKA_URI_ENV_VAR: &str = "KAFKA_URI";
    const DEFAULT_KAFKA_URI: &str = "127.0.0.1:9092";

    const QUEUE_TIMEOUT: u64 = 4;

    const KAFKA_CONFIG_ACKS: &str = "acks";
    const KAFKA_CONFIG_BATCH_SIZE: &str = "batch.size";
    const KAFKA_CONFIG_BOOTSTRAP_SERVERS: &str = "bootstrap.servers";
    const KAFKA_CONFIG_COMPRESSION_TYPE: &str = "compression.type";
    const KAFKA_CONFIG_LINGER_MS: &str = "linger.ms";

    const KAFKA_CONFIG_ACKS_DEFAULT_VALUE: &str = "1";
    const KAFKA_CONFIG_BATCH_SIZE_DEFAULT_VALUE: &str = "16384";
    const KAFKA_CONFIG_COMPRESSION_TYPE_DEFAULT_VALUE: &str = "zstd";
    const KAFKA_CONFIG_LINGER_MS_DEFAULT_VALUE: &str = "50";

    pub fn new(topic: &'static str) -> Result<Self> {
        tracing::debug!("Initializing the Kafka producer");

        let kafka_uri = std::env::var(Self::KAFKA_URI_ENV_VAR)
            .unwrap_or_else(|_| Self::DEFAULT_KAFKA_URI.to_string());

        Ok(Self {
            producer: Self::create_producer(kafka_uri)?,
            topic,
            _marker: PhantomData,
        })
    }

    fn create_producer(uri: impl AsRef<str>) -> Result<KafkaProducer> {
        let producer_config = Self::create_config(uri);

        producer_config
            .create_with_context(KafkaProducerContext)
            .map_err(|err| anyhow::anyhow!(format!("Failed to create Kafka producer: {err}")))
    }

    pub fn send(&self, payload: T) {
        let topic = self.topic;

        let serialized = match serde_json::to_string(&payload) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!("Failed to serialize message: {err}");

                MESSAGE_ERROR_COUNTER.add(1, &[opentelemetry::KeyValue::new("topic", topic)]);

                return;
            }
        };

        let producer = self.producer.clone();
        let parent_span = tracing::Span::current();
        tokio::spawn(
            async move {
                let span = tracing::info_span!("messaging.send", topic = topic);

                async {
                    tracing::debug!("Sending message");

                    let future_record: rdkafka::producer::FutureRecord<'_, str, _> =
                        if should_instrument_kafka() {
                            let mut context_injector = KafkaHeaderContextInjector::default();
                            opentelemetry::global::get_text_map_propagator(|propagator| {
                                let opentelemetry_context = tracing::Span::current().context();
                                propagator
                                    .inject_context(&opentelemetry_context, &mut context_injector);
                            });

                            let headers = rdkafka::message::OwnedHeaders::from(context_injector);

                            rdkafka::producer::FutureRecord::to(topic)
                                .payload(&serialized)
                                .headers(headers)
                        } else {
                            rdkafka::producer::FutureRecord::to(topic).payload(&serialized)
                        };

                    if let Err(err) = producer
                        .send(
                            future_record,
                            tokio::time::Duration::from_secs(Self::QUEUE_TIMEOUT),
                        )
                        .await
                        .map_err(|(kafka_error, _borrowed_message)| kafka_error)
                    {
                        tracing::error!("Failed to send message to Kafka: {err}");

                        MESSAGE_ERROR_COUNTER
                            .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);
                    } else {
                        tracing::debug!("Message sent to Kafka");

                        MESSAGE_SENT_COUNTER
                            .add(1, &[opentelemetry::KeyValue::new("topic", topic)]);
                    }
                }
                .instrument(span)
                .await;
            }
            .instrument(parent_span),
        );
    }

    fn create_config(uri: impl AsRef<str>) -> rdkafka::ClientConfig {
        let mut producer_config = rdkafka::config::ClientConfig::new();

        producer_config.set(Self::KAFKA_CONFIG_BOOTSTRAP_SERVERS, uri.as_ref());
        producer_config.set(
            Self::KAFKA_CONFIG_ACKS,
            Self::KAFKA_CONFIG_ACKS_DEFAULT_VALUE,
        );
        producer_config.set(
            Self::KAFKA_CONFIG_COMPRESSION_TYPE,
            Self::KAFKA_CONFIG_COMPRESSION_TYPE_DEFAULT_VALUE,
        );
        producer_config.set(
            Self::KAFKA_CONFIG_LINGER_MS,
            Self::KAFKA_CONFIG_LINGER_MS_DEFAULT_VALUE,
        );
        producer_config.set(
            Self::KAFKA_CONFIG_BATCH_SIZE,
            Self::KAFKA_CONFIG_BATCH_SIZE_DEFAULT_VALUE,
        );
        producer_config.set_log_level(rdkafka::config::RDKafkaLogLevel::Info);

        producer_config
    }
}
