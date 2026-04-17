pub mod context;
pub mod opentelemetry;

pub const APPLICATION_NAME: &str = "client-application";

macro_rules! counter {
    ($name:ident, $metric:literal, $description:literal) => {
        static $name: std::sync::LazyLock<::opentelemetry::metrics::Counter<u64>> =
            std::sync::LazyLock::new(|| {
                ::opentelemetry::global::meter($crate::application::APPLICATION_NAME)
                    .u64_counter($metric)
                    .with_description($description)
                    .build()
            });
    };
}

pub(crate) use counter;
