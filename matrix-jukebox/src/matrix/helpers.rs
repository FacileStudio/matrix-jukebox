
use std::{ path::Path, };
use tracing::{debug, error, instrument};

use matrix_sdk::{
    Client, ClientBuilder,  Room,
    ruma::{
        exports::serde_json,
    },
};
use tokio::fs;

use crate::settings::{ApplicationConfig, Session};

#[instrument(level = "debug")]
pub async fn stringify_room_by_name(room: &Room) -> String {
    match room.display_name().await {
        Ok(room_name) => room_name.to_string(),
        Err(error) => {
            error!(%error, "error getting room display name");
            // Let's fallback to the room ID.
            room.room_id().to_string()
        }
    }
}

#[instrument(level = "debug")]
pub async fn persist_sync_token(session_file: &Path, sync_token: String) -> eyre::Result<()> {
    let mut user_session: Session =
        serde_json::from_reader(fs::File::open(session_file).await?.into_std().await)?;
    user_session.sync_token = Some(sync_token);
    fs::write(session_file, serde_json::to_vec(&user_session)?).await?;
    Ok(())
}
#[instrument(skip_all)]
pub async fn build_client(config: &ApplicationConfig) -> eyre::Result<ClientBuilder> {
    debug!("building client");
    Ok(Client::builder()
        .server_name_or_homeserver_url(&config.client.server_name)
        .user_agent("jukebox"))
}
