use matrix_sdk::{
    Client, Room, RoomState,
    reqwest::Url,
    ruma::events::{
        call::member::{
            ActiveFocus, ActiveLivekitFocus, Application, CallApplicationContent,
            CallMemberEventContent, CallMemberStateKey, CallScope, Focus, LivekitFocus,
            OriginalSyncCallMemberEvent,
        },
        room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
            tombstone::OriginalSyncRoomTombstoneEvent,
        },
    },
};
use tracing::{debug, error, info, instrument};

use crate::matrix::helpers::{
    PreferedFocus, get_livekit_token, get_openid_token, get_prefered_foci,
};

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

    let prefered_foci = get_prefered_foci(&client).await?;
    let PreferedFocus::Livekit(livekit_info) = prefered_foci.list.first().unwrap() else {
        error!("first focus in the prefered foci list is not a livekit SFU");
        return Ok(());
    };
    let livekit_service_url = Url::parse(&livekit_info.url)?;
    let token_response = get_openid_token(&client).await?;
    let livekit_token = get_livekit_token(&client, &room, token_response, livekit_service_url).await?;
    dbg!(livekit_token);
    let our_foci_list = prefered_foci.list.into_iter().filter_map(|elem| {
        let PreferedFocus::Livekit(livekit_info) = elem else {
            error!(element=?elem, "focus is not of type livekit");
            return None;
        };
        Some(Focus::Livekit(LivekitFocus::new(
            room.room_id().to_string(),
            livekit_info.url,
        )))
    });
    let device_id = client
        .device_id()
        .expect("a logged in client should have a device id");
    let user_id = client
        .user_id()
        .expect("a logged in client should have a user id");
    let application =
        Application::Call(CallApplicationContent::new("".to_string(), CallScope::Room));
    let given_foci_prefered = member_session.foci_preferred.clone();
    let complete_foci_list = our_foci_list.chain(given_foci_prefered).collect();
    let join_event = CallMemberEventContent::new(
        application,
        device_id.into(),
        ActiveFocus::Livekit(ActiveLivekitFocus::new()),
        complete_foci_list,
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
    error!(?event, "nothing doable with this event yet");
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
    info!(%alias, %id, reasone=tombstone.content.body, "joining new room as this one was upgraded");
    client
        .join_room_by_id(&tombstone.content.replacement_room)
        .await?;
    Ok(())
}
