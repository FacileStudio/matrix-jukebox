use settings::{ApplicationConfig, Database, Session};
use std::path::Path;
use tracing::{debug, error, info, instrument};

use matrix_sdk::{
    config::SyncSettings, encryption::CrossSigningResetAuthType, ruma::{
        api::client::{filter::FilterDefinition, uiaa},
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        },
        exports::serde_json,
    }, Client, ClientBuilder, Error, LoopCtrl, Room, RoomState
};
use rand::{Rng, distr::Alphanumeric, rng};
use tokio::fs;

mod logging;
mod settings;
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

    info!("all done!");
    Ok(())
}
#[instrument(skip(config))]
async fn restore_session(
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

#[instrument(skip_all)]
async fn build_client(config: &ApplicationConfig) -> eyre::Result<ClientBuilder> {
    debug!("building client");
    Ok(Client::builder()
        .homeserver_url(&config.client.homeserver_url)
        .user_agent("jukebox"))
}
#[instrument(skip(config))]
async fn login(
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
async fn sync(
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
#[instrument(level = "debug")]
async fn persist_sync_token(session_file: &Path, sync_token: String) -> eyre::Result<()> {
    let mut user_session: Session =
        serde_json::from_reader(fs::File::open(session_file).await?.into_std().await)?;
    user_session.sync_token = Some(sync_token);
    fs::write(session_file, serde_json::to_vec(&user_session)?).await?;
    Ok(())
}
async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) ->eyre::Result<()>{
    // We only want to log text messages in joined rooms.
    if room.state() != RoomState::Joined {
        return Ok(());
    }
    let MessageType::Text(text_content) = &event.content.msgtype else {
        return Ok(());
    };

    let room_name = stringify_room_by_name(&room).await;
    debug!(%room_name, %event.sender, text_content.body, "got message");
    if text_content.body.contains("!ping") {
    let content = RoomMessageEventContent::text_plain("pong!");
    room.send(content).await?;
    }
Ok(())
}

#[instrument(skip_all)]
async fn first_time_signature_identity_bootstrap(
    config: &ApplicationConfig,
    client: &Client,
) -> eyre::Result<()> {
    info!("Bootstrapping a new cross signing identity, press enter to continue.");
    if let Some(handle) = client.encryption().reset_cross_signing().await? {
        match handle.auth_type() {
            CrossSigningResetAuthType::Uiaa(uiaa) => {
                let mut password = uiaa::Password::new(
                    client
                        .user_id()
                        .expect("a logged in client should have a user id")
                        .to_owned()
                        .into(),
                    config.client.password.clone(),
                );
                password.session = uiaa.session.clone();
                handle
                    .auth(Some(uiaa::AuthData::Password(password)))
                    .await?;
            }
            CrossSigningResetAuthType::OAuth(oauth) => {
                error!(
                    "Bots can't currently reset their identities if they're logged in with a pure oidc setup."
                );
                error!(
                    "To reset the bot's end-to-end encryption cross-signing identity anyway, you first need to login with the bot's account through your matrix oidc provider, then approve it at {}",
                    oauth.approval_url
                );
                handle.auth(None).await?;
            }
        }
    }

    Ok(())
}
#[instrument()]
async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
)  {
    if room_member.state_key != client.user_id().expect("a logged in client should have a valid user id") {
        return ;
    }

    tokio::spawn(async move {
        let room_name = stringify_room_by_name(&room).await;
        info!(room_name, "got invited to room, joining");
        room.join()
            .await
            .expect("unable to not join rooms one was invited to");
        info!("successfully joined room");
    });
}
#[instrument(level = "debug")]
async fn stringify_room_by_name(room: &Room) -> String {
    let room_name = match room.display_name().await {
        Ok(room_name) => room_name.to_string(),
        Err(error) => {
            error!(%error, "error getting room display name");
            // Let's fallback to the room ID.
            room.room_id().to_string()
        }
    };
    room_name
}
