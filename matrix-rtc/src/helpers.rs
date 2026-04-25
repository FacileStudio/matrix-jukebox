use livekit::id::ParticipantIdentity;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, time::Duration};

use matrix_sdk::{
    Client,  OwnedServerName, Room,
    reqwest::Url,
    ruma::{
        self, OwnedDeviceId, OwnedUserId, api::client::account::request_openid_token,
        authentication::TokenType, exports::serde_json,
    },
};

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
    #[serde(with = "ruma::serde::duration::secs")]
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
    let http = client.http_client();

    // Prefer querying well-known on the Matrix server name (e.g. matrix.org),
    // then fall back to the resolved homeserver URL.
    if let Some(user_id) = client.user_id() {
        let mut server_well_known = Url::parse(&format!("https://{}", user_id.server_name()))?;
        server_well_known.set_path(".well-known/matrix/client");

        if let Ok(response) = http
            .get(server_well_known)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
        {
            let bytes = response.bytes().await?;
            return Ok(serde_json::from_slice::<PreferedFoci>(&bytes)?);
        }
    }

    let mut homeserver_well_known = client.homeserver();
    homeserver_well_known.set_path(".well-known/matrix/client");
    let bytes = http
        .get(homeserver_well_known)
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(serde_json::from_slice::<PreferedFoci>(&bytes)?)
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
    pub fn new(user_id: OwnedUserId, device_id: OwnedDeviceId) -> Self {
        Self { user_id, device_id }
    }

    pub fn user_id(&self) -> &str {
        self.user_id.as_ref()
    }

    pub fn device_id(&self) -> &str {
        self.device_id.as_ref()
    }
}
impl Display for MatrixToLivekitMembership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.user_id, self.device_id))
    }
}

impl From<MatrixToLivekitMembership> for ParticipantIdentity {
    fn from(value: MatrixToLivekitMembership) -> Self {
        ParticipantIdentity(value.to_string())
    }
}
