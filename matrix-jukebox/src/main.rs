mod logging;
mod matrix;
mod settings;

use matrix::{login, restore_session, sync};
use settings::ApplicationConfig;
use tracing::{info, instrument};

const CLIENT_SESSION_FILE_NAME: &str = "session.json";
const CLIENT_STORAGE_DB_PATH: &str = "storage.db";
#[instrument(ret, err)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    logging::init();
    let config = ApplicationConfig::load().await?;
    let data_dir = &config.storage_base_dir;
    let session_file = data_dir.join(CLIENT_SESSION_FILE_NAME);
    let (client, sync_token) = if session_file.exists() {
        info!("session file was found, proceeding to restore");
        restore_session(&config, &session_file).await?
    } else {
        (login(&config, data_dir, &session_file).await?, None)
    };

    sync(client, sync_token, &session_file).await?;
    Ok(())
}
