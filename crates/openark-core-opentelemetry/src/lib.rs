#[cfg(not(target_arch = "wasm32"))]
use std::env;

use opentelemetry::global;
#[cfg(feature = "opentelemetry-otlp")]
use opentelemetry_otlp as otlp;
#[cfg(feature = "opentelemetry-otlp")]
use opentelemetry_sdk as sdk;
#[cfg(not(target_arch = "wasm32"))]
use tracing::Level;
use tracing::{Subscriber, debug, dispatcher};
use tracing_subscriber::{
    Layer, Registry, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt,
};

#[cfg(not(target_arch = "wasm32"))]
mod consts {
    pub(super) const SERVICE_NAME_KEY: &str = "OTEL_SERVICE_NAME";
    pub(super) const SERVICE_NAME_VALUE: &str = env!("CARGO_CRATE_NAME");
}

fn init_once_opentelemetry(export: bool) {
    #[cfg(feature = "opentelemetry-otlp")]
    use sdk::runtime::Tokio as Runtime;

    // Skip init if has been set
    if dispatcher::has_been_set() {
        return;
    }

    // Set default service name
    {
        if env::var_os(consts::SERVICE_NAME_KEY).is_none() {
            unsafe { env::set_var(consts::SERVICE_NAME_KEY, consts::SERVICE_NAME_VALUE) }
        }
    }

    fn init_layer_env_filter<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let layer = ::tracing_subscriber::EnvFilter::from_default_env();
        #[cfg(target_arch = "wasm32")]
        let layer = layer.add_directive(::tracing::Level::INFO.into());
        layer
    }

    #[cfg(target_arch = "wasm32")]
    fn init_layer_perf<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        ::tracing_web::performance_layer()
            .with_details_from_fields(::tracing_subscriber::fmt::format::Pretty::default())
    }

    fn init_layer_stdfmt<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let layer = ::tracing_subscriber::fmt::layer()
            .with_timer(::tracing_subscriber::fmt::time::ChronoUtc::rfc_3339());
        #[cfg(target_arch = "wasm32")]
        let layer = layer
            .with_ansi(false)
            .with_writer(tracing_web::MakeConsoleWriter)
            .with_span_events(::tracing_subscriber::fmt::format::FmtSpan::ACTIVE);
        layer
    }

    #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-logs"))]
    fn init_layer_otlp_logger<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let exporter = otlp::LogExporter::builder()
            .with_tonic()
            .build()
            .expect("failed to init a log exporter");

        let processor = sdk::logs::log_processor_with_async_runtime::BatchLogProcessor::builder(
            exporter, Runtime,
        )
        .build();

        let provider = sdk::logs::SdkLoggerProvider::builder()
            .with_log_processor(processor)
            .build();

        ::opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&provider)
    }

    #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-metrics"))]
    fn init_layer_otlp_metrics<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let exporter = otlp::MetricExporter::builder()
            .with_tonic()
            .build()
            .expect("failed to init a metric exporter");

        // let reader = sdk::metrics::periodic_reader_with_async_runtime::PeriodicReader::builder(
        //     exporter, Runtime,
        // )
        // .build();
        let reader = sdk::metrics::PeriodicReader::builder(exporter).build();

        let provider = sdk::metrics::MeterProviderBuilder::default()
            .with_reader(reader)
            .build();

        let layer = ::tracing_opentelemetry::MetricsLayer::new(provider.clone());
        global::set_meter_provider(provider);
        layer
    }

    #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-trace"))]
    fn init_layer_otlp_tracer<S>() -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        use opentelemetry::trace::TracerProvider;

        let name = env::var(consts::SERVICE_NAME_KEY)
            .unwrap_or_else(|_| consts::SERVICE_NAME_VALUE.into());

        let exporter = otlp::SpanExporter::builder()
            .with_tonic()
            .build()
            .expect("failed to init a span exporter");

        let processor = sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor::builder(
            exporter, Runtime,
        )
        .build();

        let provider = sdk::trace::SdkTracerProvider::builder()
            .with_span_processor(processor)
            .build();

        let layer = ::tracing_opentelemetry::OpenTelemetryLayer::new(provider.tracer(name));
        global::set_tracer_provider(provider);
        layer
    }

    let layer = Registry::default()
        .with(init_layer_env_filter())
        .with(init_layer_stdfmt());

    #[cfg(target_arch = "wasm32")]
    let layer = layer.with(init_layer_perf());

    let is_otel_exporter_activated = {
        #[cfg(target_arch = "wasm32")]
        {
            false
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
        }
    };
    if export && is_otel_exporter_activated {
        #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-logs"))]
        let layer = layer.with(init_layer_otlp_logger());
        #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-metrics"))]
        let layer = layer.with(init_layer_otlp_metrics());
        #[cfg(all(feature = "opentelemetry-otlp", feature = "opentelemetry-trace"))]
        let layer = layer.with(init_layer_otlp_tracer());

        layer.init()
    } else {
        if export && !is_otel_exporter_activated {
            debug!("OTEL exporter is not activated.");
        }

        layer.init()
    }
}

pub fn init_once() {
    #[cfg(target_arch = "wasm32")]
    {
        init_once_opentelemetry(false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        init_once_with_default(true)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_once_with(level: Level, export: bool) {
    // Skip init if has been set
    if dispatcher::has_been_set() {
        return;
    }

    // set custom tracing level
    unsafe { env::set_var(KEY, level.as_str()) };

    init_once_opentelemetry(export)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_once_with_default(export: bool) {
    // Skip init if has been set
    if dispatcher::has_been_set() {
        return;
    }

    // set default tracing level
    if env::var_os(KEY).is_none() {
        unsafe { env::set_var(KEY, "INFO") };
    }

    init_once_opentelemetry(export)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_once_with_level_int(level: u8, export: bool) {
    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    let level = match level {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        3 => Level::TRACE,
        level => panic!("too high debug level: {level}"),
    };
    init_once_with(level, export)
}

#[cfg(not(target_arch = "wasm32"))]
const KEY: &str = "RUST_LOG";
