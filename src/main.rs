use tracing::info;
mod logging;
#[tracing::instrument(ret, err)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    logging::init();
    info!("Hello, world!");
    Ok(())
}
