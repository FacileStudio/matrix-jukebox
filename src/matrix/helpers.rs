use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tracing::{debug, error, instrument};

use matrix_sdk::{Client, ClientBuilder, Room, reqwest::Url, ruma::exports::serde_json};
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum PreferedFocus {
    Livekit(LivekitInformation),
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LivekitInformation {
    #[serde(rename = "livekit_service_url")]
    pub url: String,
}
#[derive(Debug, Serialize, Deserialize)]
// #[serde(transparent)]
pub struct PreferedFoci {
    #[serde(rename = "org.matrix.msc4143.rtc_foci")]
    pub list: Vec<PreferedFocus>,
}
pub async fn get_prefered_foci(
    client: &Client,
    config: &ApplicationConfig,
) -> eyre::Result<PreferedFoci> {
    let client = client.http_client();
    let url = if !config.client.server_name.starts_with("http")
        || !config.client.server_name.starts_with("https")
    {
        let mut value = config.client.server_name.clone();
        value.insert_str(0, "https");
        value
    } else {
        config.client.server_name.clone()
    };
    let mut url = Url::parse(&url)?;
    url.set_path(".well-known/matrix/client");
    Ok(serde_json::from_slice::<PreferedFoci>(
        &client
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?,
    )?)
}
