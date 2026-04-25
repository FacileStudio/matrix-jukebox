use eyre::{Context, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
mod bot;
mod client;
mod db;
mod session;
use bot::*;
use client::*;
pub use db::*;
pub use session::*;
use tracing::{error, info, instrument};
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub bot: Bot,
    #[serde(default)]
    pub client: Client,
    pub storage_base_dir: PathBuf,
}

fn apply_env_override(target: &mut String, env_keys: &[&str]) {
    for key in env_keys {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                *target = value;
                break;
            }
        }
    }
}

fn ensure_non_empty(field_name: &str, value: &str, env_keys: &[&str]) -> eyre::Result<()> {
    if value.trim().is_empty() {
        bail!(
            "missing configuration for `{field_name}`. Set it in config.yaml or one of: {}",
            env_keys.join(", ")
        );
    }

    Ok(())
}

impl ApplicationConfig {
    #[instrument]
    pub async fn load() -> eyre::Result<Self> {
        const DEFAULT_PATH_LOCATION: &str = "config.yaml";
        let config_path: PathBuf = std::env::var("CONFIG_PATH")
            .or_else(|_| std::env::var("MATRIX_JUKEBOX_CONFIG_PATH"))
            .unwrap_or_else(|e| {
                error!(error=%e, "unable to find CONFIG_PATH or MATRIX_JUKEBOX_CONFIG_PATH, falling back to the default of {DEFAULT_PATH_LOCATION}");
                DEFAULT_PATH_LOCATION.to_owned()
            })
            .parse()?;
        info!(?config_path, "this is the full config");
        let mut config: ApplicationConfig = serde_yaml::from_reader(
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

        apply_env_override(
            &mut config.client.server_name,
            &["MATRIX_SERVER_NAME", "MATRIX_JUKEBOX_SERVER_NAME"],
        );
        apply_env_override(
            &mut config.client.user_name,
            &["MATRIX_USER_NAME", "MATRIX_JUKEBOX_USER_NAME"],
        );
        apply_env_override(
            &mut config.client.password,
            &["MATRIX_PASSWORD", "MATRIX_JUKEBOX_PASSWORD"],
        );

        ensure_non_empty(
            "client.server_name",
            &config.client.server_name,
            &["MATRIX_SERVER_NAME", "MATRIX_JUKEBOX_SERVER_NAME"],
        )?;
        ensure_non_empty(
            "client.user_name",
            &config.client.user_name,
            &["MATRIX_USER_NAME", "MATRIX_JUKEBOX_USER_NAME"],
        )?;
        ensure_non_empty(
            "client.password",
            &config.client.password,
            &["MATRIX_PASSWORD", "MATRIX_JUKEBOX_PASSWORD"],
        )?;

        Ok(config)
    }
}
