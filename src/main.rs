use settings::ApplicationConfig;
use tracing::info;
mod logging;
mod settings;
#[tracing::instrument(ret, err)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    logging::init();
    let config = ApplicationConfig::load().await?;
    info!(?config.storage_base_dir, ?config, "got configuration");
    info!("Hello, world!");
    Ok(())
}
