use std::path::Path;
use tracing::error;

use matrix_sdk::{Room, ruma::exports::serde_json};
use tokio::fs;
use tracing::instrument;

use crate::settings::Session;

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
