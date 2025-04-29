use std::env::VarError;

use color_eyre::Result;
use tracing::Subscriber;
use tracing_error::ErrorLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::env;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOG_LEVEL", env::PROJECT_NAME.to_uppercase().clone());
}

/// An RAII guard that executes the stored function on drop.
pub struct OnDrop(Option<Box<dyn FnOnce()>>);

impl OnDrop {
    #[allow(unused)]
    pub fn new(on_drop: Box<dyn FnOnce()>) -> Self {
        Self(Some(on_drop))
    }
}

impl Drop for OnDrop {
    fn drop(&mut self) {
        if let Some(on_drop) = self.0.take() {
            (on_drop)();
        }
    }
}

/// An RAII guard that takes care of shutting down all of tracing-related services on drop.
#[derive(Default)]
pub struct TracingGuard {
    #[allow(unused)]
    on_drop: Vec<OnDrop>,
}

#[cfg(feature = "opentelemetry")]
mod opentelemetry {
    use super::*;
    use ::opentelemetry::trace::TracerProvider;
    use ::opentelemetry_otlp::{Protocol, WithExportConfig};
    use ::opentelemetry_sdk::Resource;
    use ::opentelemetry_sdk::metrics::SdkMeterProvider;
    use ::opentelemetry_sdk::trace::SdkTracerProvider;
    use ::std::time::Duration;

    pub fn create_tracer_layer<S>(
        tracing_guard: &mut TracingGuard,
    ) -> Result<tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(Duration::from_secs(3))
            .build()?;
        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter)
            .with_resource(
                Resource::builder()
                    .with_service_name(env::PKG_NAME.to_string())
                    .build(),
            )
            .build();
        let tracer = tracer_provider.tracer(&*env::PKG_NAME);
        let layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);

        tracing_guard.on_drop.push(OnDrop::new(Box::new(move || {
            tracer_provider.shutdown().unwrap();
        })));

        Ok(layer)
    }

    pub fn create_meter_layer<S>(
        tracing_guard: &mut TracingGuard,
    ) -> Result<tracing_opentelemetry::MetricsLayer<S>>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let otel_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(Duration::from_secs(3))
            .build()?;
        let meter_provider = SdkMeterProvider::builder()
            .with_periodic_exporter(otel_exporter)
            .with_resource(
                Resource::builder()
                    .with_service_name(env::PKG_NAME.to_string())
                    .build(),
            )
            .build();
        // Clone is a shallow copy of a smart pointer.
        let layer = tracing_opentelemetry::MetricsLayer::new(meter_provider.clone());

        tracing_guard.on_drop.push(OnDrop::new(Box::new(move || {
            meter_provider.shutdown().unwrap();
        })));

        Ok(layer)
    }
}

#[cfg(feature = "tracy")]
mod tracy {
    use super::*;
    use ::tracing_subscriber::fmt::format::DefaultFields;

    #[derive(Default)]
    pub struct TracyLayerConfig {
        fmt: DefaultFields,
    }

    impl tracing_tracy::Config for TracyLayerConfig {
        type Formatter = DefaultFields;
        fn formatter(&self) -> &Self::Formatter {
            &self.fmt
        }
        // The boilerplate ends here

        /// Collect 32 frames in stack traces.
        fn stack_depth(&self, _: &tracing::Metadata) -> u16 {
            32
        }
    }

    pub fn create_layer(
        _tracing_guard: &mut TracingGuard,
    ) -> Result<tracing_tracy::TracyLayer<TracyLayerConfig>> {
        let tracy_layer = tracing_tracy::TracyLayer::new(TracyLayerConfig::default());

        Ok(tracy_layer)
    }
}

pub fn create_file_layer<S>(
    log_path: String,
    _tracing_guard: &mut TracingGuard,
) -> Result<impl tracing_subscriber::layer::Layer<S>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;
    let log_file = std::fs::File::create(log_path)?;
    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);
    Ok(file_subscriber)
}

/// Enable logging if the `LOG_FILE` environment variable is specified.
pub fn init() -> Result<TracingGuard> {
    let mut tracing_guard = TracingGuard::default();
    let subscriber = tracing_subscriber::registry();

    match std::env::var("LOG_FILE") {
        Ok(log_path) => with_rest(
            subscriber.with(create_file_layer(log_path, &mut tracing_guard)?),
            &mut tracing_guard,
        )?,
        Err(VarError::NotPresent) => with_rest(subscriber, &mut tracing_guard)?,
        Err(err) => return Err(err.into()),
    }

    Ok(tracing_guard)
}

fn with_rest<S>(
    subscriber: S,
    #[cfg_attr(
        all(not(feature = "opentelemetry"), not(feature = "tracy")),
        expect(unused_variables)
    )]
    tracing_guard: &mut TracingGuard,
) -> Result<()>
where
    S: Subscriber + Send + Sync + 'static + SubscriberInitExt + for<'span> LookupSpan<'span>,
{
    let subscriber = subscriber.with(ErrorLayer::default());

    #[cfg(feature = "opentelemetry")]
    let subscriber = subscriber
        .with(self::opentelemetry::create_tracer_layer(tracing_guard)?)
        .with(self::opentelemetry::create_meter_layer(tracing_guard)?);

    #[cfg(feature = "tracy")]
    let subscriber = subscriber.with(self::tracy::create_layer(tracing_guard)?);

    subscriber.try_init()?;
    Ok(())
}

#[macro_export]
macro_rules! tracing_dbg {
    ($expr:expr) => {{
        let result = $expr;
        ::tracing::debug!("{} = {result:#?}", stringify!($expr));
        result
    }};
}
