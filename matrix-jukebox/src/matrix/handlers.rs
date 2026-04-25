use matrix_sdk::{
    Client, Room, RoomState,
    event_handler::Ctx,
    ruma::events::room::{
        member::StrippedRoomMemberEvent,
        message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        tombstone::OriginalSyncRoomTombstoneEvent,
    },
};
use tracing::{debug, info, instrument};

use matrix_rtc::rtc_session_manager::MatrixRtcSessionManager;

use super::helpers::stringify_room_by_name;

pub async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    rtc: Ctx<MatrixRtcSessionManager>,
    command_prefix: Ctx<String>,
) -> eyre::Result<()> {
    // We only want to log text messages in joined rooms.
    if room.state() != RoomState::Joined {
        return Ok(());
    }
    let MessageType::Text(text_content) = &event.content.msgtype else {
        return Ok(());
    };

    let room_name = stringify_room_by_name(&room).await;
    debug!(%room_name, %event.sender, text_content.body, "got message");

    let body = text_content.body.trim();
    let Some(command_body) = body.strip_prefix(command_prefix.as_str()) else {
        return Ok(());
    };
    let mut command_parts = command_body.trim_start().splitn(2, char::is_whitespace);
    let Some(command_name) = command_parts.next() else {
        return Ok(());
    };

    match command_name {
        "ping" => {
            let content = RoomMessageEventContent::text_plain("pong!");
            room.send(content).await?;
        }

        "play" => {
            let Some(url) = command_parts.next().map(str::trim).filter(|s| !s.is_empty()) else {
                // !play was sent without a url, send usage instructions
                room.send(RoomMessageEventContent::text_plain(
                    "usage: !play <youtube-url>",
                ))
                .await?;
                return Ok(());
            };

            if let Err(error) = rtc
                .play_youtube_url(room.room_id().to_owned(), url.to_owned())
                .await
            {
                room.send(RoomMessageEventContent::text_plain(format!(
                    "unable to queue playback: {error}",
                )))
                .await?;
                return Ok(());
            }
            room.send(RoomMessageEventContent::text_plain(format!(
                "queued playback for {url}",
            )))
            .await?;
        }
        _ => {}
    }
    Ok(())
}

#[instrument()]
pub async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    if room_member.state_key
        != client
            .user_id()
            .expect("a logged in client should have a valid user id")
    {
        return;
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

#[instrument(skip_all)]
pub async fn on_room_upgrade(
    tombstone: OriginalSyncRoomTombstoneEvent,
    client: Client,
    room: Room,
) -> eyre::Result<()> {
    let alias = room
        .canonical_alias()
        .map_or_else(|| "no alias provided".to_owned(), |id| id.to_string());
    let id = room.room_id();
    info!(%alias, %id, reason=tombstone.content.body, "joining new room as this one was upgraded");
    client
        .join_room_by_id(&tombstone.content.replacement_room)
        .await?;
    Ok(())
}
