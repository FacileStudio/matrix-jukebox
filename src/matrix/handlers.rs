use matrix_sdk::{
    Client, Room, RoomState,
    ruma::events::{
        call::member::{
            ActiveFocus, ActiveLivekitFocus, CallMemberEventContent, CallMemberStateKey,
            OriginalSyncCallMemberEvent,
        },
        room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        },
    },
};
use tracing::{debug, error, info, instrument};

use super::{custom_events::EncryptionKeysChangedEvent, helpers::stringify_room_by_name};

pub async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) -> eyre::Result<()> {
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
pub async fn on_rtc_member_join(
    member: OriginalSyncCallMemberEvent,
    client: Client,
    room: Room,
) -> eyre::Result<()> {
    info!(?member, "recieved event!");
    if member.sender
        == client
            .user_id()
            .expect("a logged in client should have a user id")
    {
        return Ok(());
    }
    let member_session = match &member.content {
        CallMemberEventContent::LegacyContent(_) => {
            error!("we don't support legacy matrix rtc sessions");
            return Ok(());
        }
        CallMemberEventContent::SessionContent(session_membership_data) => session_membership_data,
        CallMemberEventContent::Empty(_) => {
            let member_name = member.sender.localpart();
            info!("{member_name} left the call");
            return Ok(());
        }
        kind => {
            error!(?kind, "we don't know what to do with this");
            return Ok(());
        }
    };
    let application = &member_session.application;
    let device_id = client
        .device_id()
        .expect("a logged in client should have a device id");
    let user_id = client
        .user_id()
        .expect("a logged in client should have a user id");

    let foci_prefered = &member_session.foci_preferred;
    let join_event = CallMemberEventContent::new(
        application.clone(),
        device_id.into(),
        ActiveFocus::Livekit(ActiveLivekitFocus::new()),
        foci_prefered.to_vec(),
        None,
    );
    let leave_event = CallMemberEventContent::new_empty(None);
    let state_key = CallMemberStateKey::new(user_id.into(), Some(device_id.into()), true);
    room.send_state_event_for_key(&state_key, join_event)
        .await?;
    client.add_event_handler(on_rtc_encryption_key_changed_event);
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    room.send_state_event_for_key(&state_key, leave_event)
        .await?;
    Ok(())
}
#[instrument]
pub async fn on_rtc_encryption_key_changed_event(
    event: EncryptionKeysChangedEvent,
    client: Client,
) {
    info!(?event, "got this event. What next?");
}
