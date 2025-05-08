use eyre::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
mod bot;
mod client;
mod db;
mod session;
use bot::*;
use client::*;
use tracing::{Instrument, error, info, instrument};
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub bot: Bot,
    pub client: Client,
    pub storage_base_dir: PathBuf,
}
impl ApplicationConfig {
    #[instrument]
    pub async fn load() -> eyre::Result<Self> {
        const DEFAULT_PATH_LOCATION: &str = "config.yaml";
        let mut config_path: PathBuf = std::env::var("CONFIG_PATH").unwrap_or_else(|e|{
            error!(error=%e, "unable to find the environment variable CONFIG_PATH, falling back to the default of {DEFAULT_PATH_LOCATION}");
            DEFAULT_PATH_LOCATION.to_owned()
        }).parse()?;
        info!(?config_path, "this is the full config");
        let config: ApplicationConfig = serde_yaml::from_reader(
            fs::File::open(&config_path)
                .await
                .context("unable to open file")?
                .into_std()
                .await,
        )
        .context(format!(
            "unable to deserialize configuration file {:?}",
            &config_path
        ))?;
        Ok(config)
    }
}
