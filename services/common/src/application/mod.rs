pub mod opentelemetry;

use std::sync::OnceLock;

static APPLICATION_NAME: OnceLock<&'static str> = OnceLock::new();

pub fn set_application_name(name: &'static str) {
    let _ = APPLICATION_NAME.set(name);
}

pub fn application_name() -> &'static str {
    APPLICATION_NAME.get().copied().unwrap_or("unknown")
}

#[macro_export]
macro_rules! counter {
    ($name:ident, $metric:literal, $description:literal) => {
        static $name: ::std::sync::LazyLock<::opentelemetry::metrics::Counter<u64>> =
            ::std::sync::LazyLock::new(|| {
                ::opentelemetry::global::meter($crate::application::application_name())
                    .u64_counter($metric)
                    .with_description($description)
                    .build()
            });
    };
}
