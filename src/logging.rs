use std::env::VarError;
use std::time::Duration;

use color_eyre::Result;
use tracing::Subscriber;
use tracing_error::ErrorLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::config;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOG_LEVEL", config::PROJECT_NAME.clone());
}

#[cfg(feature = "opentelemetry")]
mod opentelemetry {
    use super::*;
    use ::opentelemetry::trace::TracerProvider;
    use ::opentelemetry_otlp::{Protocol, WithExportConfig};
    use ::opentelemetry_sdk::trace::SdkTracerProvider;
    use ::opentelemetry_sdk::Resource;

    pub fn create_layer<S>(
    ) -> Result<tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let otel_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(Duration::from_secs(3))
            .build()?;
        let otel_provider = SdkTracerProvider::builder()
            .with_batch_exporter(otel_exporter)
            .with_resource(
                Resource::builder()
                    .with_service_name(config::PROJECT_NAME.to_string())
                    .build(),
            )
            .build();
        let otel_tracer = otel_provider.tracer(&*config::PROJECT_NAME);
        let otel_layer = tracing_opentelemetry::layer::<S>().with_tracer(otel_tracer);

        Ok(otel_layer)
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

    pub fn create_layer() -> Result<tracing_tracy::TracyLayer<TracyLayerConfig>> {
        let tracy_layer = tracing_tracy::TracyLayer::new(TracyLayerConfig::default());

        Ok(tracy_layer)
    }
}

/// Enable logging if the `LOG_FILE` environment variable is specified.
pub fn init() -> Result<()> {
    // let directory = config::get_data_dir();
    // std::fs::create_dir_all(directory.clone())?;
    let log_path = match std::env::var("LOG_FILE") {
        Ok(log_file) => log_file,
        Err(VarError::NotPresent) => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    let log_file = std::fs::File::create(log_path)?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    let subscriber = tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default());

    #[cfg(feature = "opentelemetry")]
    let subscriber = subscriber.with(self::opentelemetry::create_layer()?);

    #[cfg(feature = "tracy")]
    let subscriber = subscriber.with(self::tracy::create_layer()?);

    subscriber.try_init()?;
    Ok(())
}
