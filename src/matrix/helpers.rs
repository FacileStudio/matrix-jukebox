use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tracing::{debug, error, instrument};

use matrix_sdk::{
    Client, ClientBuilder, OwnedServerName, Room,
    reqwest::Url,
    ruma::{
        OwnedDeviceId, OwnedUserId, api::client::account::request_openid_token,
        authentication::TokenType, exports::serde_json,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum PreferedFocus {
    Livekit(LivekitInformation),
    #[serde(untagged)]
    Unknown(String),
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LivekitInformation {
    #[serde(rename = "livekit_service_url")]
    pub url: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PreferedFoci {
    #[serde(rename = "org.matrix.msc4143.rtc_foci")]
    pub list: Vec<PreferedFocus>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct LivekitJwtRequest {
    device_id: OwnedDeviceId,
    room: String,
    openid_token: OpenIDTokenResponse,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenIDTokenResponse {
    access_token: String,
    expires_in: Duration,
    token_type: TokenType,
    matrix_server_name: OwnedServerName,
}
#[derive(Deserialize, Debug)]
pub struct LivekitTokenResponse {
    #[serde(rename = "jwt")]
    pub token: String,
    pub url: Url,
}
impl From<request_openid_token::v3::Response> for OpenIDTokenResponse {
    fn from(value: request_openid_token::v3::Response) -> Self {
        Self {
            access_token: value.access_token,
            expires_in: value.expires_in,
            token_type: value.token_type,
            matrix_server_name: value.matrix_server_name,
        }
    }
}
pub async fn get_prefered_foci(client: &Client) -> eyre::Result<PreferedFoci> {
    let mut url = client.homeserver();
    let client = client.http_client();
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

pub async fn get_openid_token(client: &Client) -> eyre::Result<OpenIDTokenResponse> {
    let request = request_openid_token::v3::Request::new(
        client
            .user_id()
            .expect("a logged in client should have a user id")
            .into(),
    );
    Ok(client.send(request).await?.into())
}
pub async fn get_livekit_token(
    client: &Client,
    room: &Room,
    token_response: OpenIDTokenResponse,
    livekit_service_url: Url,
) -> eyre::Result<LivekitTokenResponse> {
    let mut livekit_service_url = livekit_service_url;
    livekit_service_url
        .path_segments_mut()
        .expect("url cannot be a base without a path")
        .push("sfu")
        .push("get");
    let request = LivekitJwtRequest {
        device_id: client
            .device_id()
            .expect("a logged in client should have a device id")
            .into(),
        room: room.room_id().to_string(),
        openid_token: token_response,
    };
    let jwt_service_response_bytes: &[u8] = &client
        .http_client()
        .post(livekit_service_url)
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(10))
        .body(serde_json::to_string(&request)?)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(serde_json::from_slice(jwt_service_response_bytes)?)
}

pub struct MatrixToLivekitMembership {
    user_id: OwnedUserId,
    device_id: OwnedDeviceId,
}

impl MatrixToLivekitMembership {
    #[expect(dead_code)]
    pub fn new(user_id: OwnedUserId, device_id: OwnedDeviceId) -> Self {
        Self { user_id, device_id }
    }

    #[expect(dead_code)]
    pub fn user_id(&self) -> &str {
        self.user_id.as_ref()
    }

    #[expect(dead_code)]
    pub fn device_id(&self) -> &str {
        self.device_id.as_ref()
    }
}
impl ToString for MatrixToLivekitMembership {
    fn to_string(&self) -> String {
        format!(
            "{}:{}",
            self.user_id.to_string(),
            self.device_id.to_string()
        )
    }
}
