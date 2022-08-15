use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() -> WorkerGuard {
    let (file_appender, file_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(".", format!("{}.log", env!("CARGO_PKG_NAME"))),
    );

    tracing_subscriber::Registry::default()
        .with(EnvFilter::new(format!("{}=debug", env!("CARGO_PKG_NAME"))))
        // file logger settings
        .with(layer().with_ansi(false).with_writer(file_appender))
        // console logger settings
        .with(tracing_subscriber::fmt::layer().with_ansi(true).pretty())
        .init();

    file_guard
}
