mod logging;
mod matrix;
mod settings;

use matrix::{login, restore_session, sync};
use settings::ApplicationConfig;
use tokio::fs;
use tracing::{info, instrument, warn};

const CLIENT_SESSION_FILE_NAME: &str = "session.json";
const CLIENT_STORAGE_DB_PATH: &str = "storage.db";
#[instrument(ret, err)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    logging::init();
    let config = ApplicationConfig::load().await?;
    let data_dir = &config.storage_base_dir;
    fs::create_dir_all(data_dir).await?;
    let session_file = data_dir.join(CLIENT_SESSION_FILE_NAME);
    let (client, sync_token) = if session_file.exists() {
        info!("session file was found, proceeding to restore");
        restore_session(&config, &session_file).await?
    } else {
        // No session file means no known passphrase. Remove any orphaned DB
        // (e.g. from a previous crash before session.json was written) so the
        // fresh login doesn't try to open it with a new random passphrase.
        let _ = fs::remove_dir_all(data_dir.join(CLIENT_STORAGE_DB_PATH)).await;
        (login(&config, data_dir, &session_file).await?, None)
    };

    if let Err(error) = sync(
        client,
        sync_token,
        &session_file,
        config.bot.command_prefix.clone(),
    )
    .await
    {
        let error_text = error.to_string();
        if error_text.contains("M_UNKNOWN_TOKEN") || error_text.contains("Token is not active") {
            warn!("session is no longer valid, clearing local session and re-authenticating");
            let _ = fs::remove_file(&session_file).await;
            let _ = fs::remove_dir_all(data_dir.join(CLIENT_STORAGE_DB_PATH)).await;
            let client = login(&config, data_dir, &session_file).await?;
            sync(client, None, &session_file, config.bot.command_prefix.clone()).await?;
        } else {
            return Err(error);
        }
    }
    Ok(())
}
