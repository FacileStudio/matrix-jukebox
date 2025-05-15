use std::path::Path;

use encryption::first_time_signature_identity_bootstrap;
use handlers::{on_room_message, on_rtc_member_join, on_stripped_state_member};
use helpers::{build_client, persist_sync_token};
use matrix_sdk::{
    Client, Error, LoopCtrl,
    config::SyncSettings,
    ruma::{api::client::filter::FilterDefinition, exports::serde_json},
};
use rand::{Rng, distr::Alphanumeric, rng};
use tokio::fs;
use tracing::{debug, info, instrument};

use crate::{
    CLIENT_STORAGE_DB_PATH,
    settings::{ApplicationConfig, Database, Session},
};

mod custom_events;
mod encryption;
mod handlers;
mod helpers;
#[instrument(skip_all)]
pub async fn sync(
    client: Client,
    initial_sync_token: Option<String>,
    session_file: &Path,
) -> eyre::Result<()> {
    let filter = FilterDefinition::with_lazy_loading();

    let mut sync_settings = SyncSettings::default().filter(filter.into());

    if let Some(sync_token) = initial_sync_token {
        sync_settings = sync_settings.token(sync_token);
    }
    client.add_event_handler(on_stripped_state_member);
    let response = client.sync_once(sync_settings.clone()).await?;
    sync_settings = sync_settings.token(response.next_batch.clone());
    persist_sync_token(session_file, response.next_batch).await?;
    client.add_event_handler(on_room_message);
    client.add_event_handler(on_rtc_member_join);
    client
        .sync_with_result_callback(sync_settings, |sync_result| {
            async move {
                let response = sync_result?;
                // We persist the token each time to be able to restore our session
                persist_sync_token(session_file, response.next_batch)
                    .await
                    .map_err(|err| Error::UnknownError(err.into()))?;
                Ok(LoopCtrl::Continue)
            }
        })
        .await?;
    Ok(())
}
#[instrument(skip(config))]
pub async fn restore_session(
    config: &ApplicationConfig,
    session_file_path: &Path,
) -> eyre::Result<(Client, Option<String>)> {
    let serialized_session = fs::read_to_string(session_file_path).await?;
    let session: Session = serde_json::from_str(&serialized_session)?;
    let client = build_client(config)
        .await?
        .sqlite_store(
            config.storage_base_dir.join(CLIENT_STORAGE_DB_PATH),
            Some(&session.database.passphrase),
        )
        .build()
        .await?;
    client.restore_session(session.user_session).await?;
    Ok((client, session.sync_token))
}

#[instrument(skip(config))]
pub async fn login(
    config: &ApplicationConfig,
    data_dir: &Path,
    session_file: &Path,
) -> eyre::Result<Client> {
    info!("no session file found, logging in");
    debug!("using random generation to create a passphrase for the sqlite store database");
    let passphrase: String = rng()
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let client = build_client(config)
        .await?
        .sqlite_store(data_dir.join(CLIENT_STORAGE_DB_PATH), Some(&passphrase))
        .build()
        .await?;
    let auth = client.matrix_auth();
    auth.login_username(&config.client.user_name, &config.client.password)
        .initial_device_display_name("jukebox")
        .await?;
    let matrix_session = auth
        .session()
        .expect("a logged in client should have a session");
    let session = Session {
        user_session: matrix_session,
        database: Database { passphrase },
        sync_token: None,
    };
    let serialized_session = serde_json::to_string(&session)?;
    fs::write(session_file, serialized_session).await?;
    info!("login complete, bootstrapping cross-signing identity");
    first_time_signature_identity_bootstrap(config, &client).await?;
    Ok(client)
}
