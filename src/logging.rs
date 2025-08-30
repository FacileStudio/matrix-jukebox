use std::env;

use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, prelude::*};
use tracing_tree::HierarchicalLayer;

const DEFAULT_LOG_FILTER: &str = "matrix-jukebox=debug,warn";

/// Initialise the logging stack.
pub fn init() {
    let env_filter = match env::var("JUKEBOX_LOG").or_else(|_| env::var("RUST_LOG")) {
        Ok(s) => EnvFilter::from(s),
        Err(env::VarError::NotPresent) => EnvFilter::from(DEFAULT_LOG_FILTER),
        Err(e) => {
            eprintln!("Warning: Failed to read log environment variables. Error cause is: {e}");
            EnvFilter::from(DEFAULT_LOG_FILTER)
        }
    };
    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(ErrorLayer::default())
        .with(
            HierarchicalLayer::new(4)
                .with_ansi(false)
                .with_bracketed_fields(true),
        );
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Warning: Failed to set log handler: {e}");
    }
    if let Err(e) = LogTracer::init() {
        tracing::warn!(error = %e, "Failed to install log facade");
    }
}
