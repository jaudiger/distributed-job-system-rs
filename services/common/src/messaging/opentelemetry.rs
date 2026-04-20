use rdkafka::message::Headers as _;
use std::sync::LazyLock;

static ENABLE_OTEL_TRACES: LazyLock<bool> = LazyLock::new(|| {
    match std::env::var("OTEL_TRACES_SAMPLER") {
        Ok(value) if (value == "always_off") || (value == "parentbased_always_off") => {
            return false;
        }
        _ => {}
    }

    !matches!(std::env::var("OTEL_TRACES_SAMPLER_ARG"), Ok(value) if value == "0")
});

#[inline]
pub fn should_instrument_kafka() -> bool {
    *ENABLE_OTEL_TRACES
}

#[derive(Default)]
pub struct KafkaHeaderContextInjector(Vec<(String, String)>);

impl From<KafkaHeaderContextInjector> for rdkafka::message::OwnedHeaders {
    fn from(value: KafkaHeaderContextInjector) -> Self {
        let mut headers = Self::new();
        for (key, value) in value.0 {
            headers = headers.insert(rdkafka::message::Header {
                key: key.as_str(),
                value: Some(value.as_str()),
            });
        }

        headers
    }
}

impl opentelemetry::propagation::Injector for KafkaHeaderContextInjector {
    fn set(&mut self, key: &str, value: String) {
        self.0.push((key.to_string(), value));
    }
}

pub struct KafkaHeaderContextExtractor<'a>(Option<&'a rdkafka::message::BorrowedHeaders>);

impl<'a> KafkaHeaderContextExtractor<'a> {
    pub const fn new(headers: Option<&'a rdkafka::message::BorrowedHeaders>) -> Self {
        Self(headers)
    }
}

impl opentelemetry::propagation::Extractor for KafkaHeaderContextExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        if let Some(headers) = self.0 {
            for header in headers.iter() {
                if header.key == key
                    && let Some(bytes) = header.value
                    && let Ok(string) = std::str::from_utf8(bytes)
                {
                    return Some(string);
                }
            }
        }
        None
    }

    fn keys(&self) -> Vec<&str> {
        self.0.map_or_else(Vec::new, |headers| {
            headers.iter().map(|header| header.key).collect()
        })
    }
}
