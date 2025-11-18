use std::collections::BTreeMap;

use eyre::eyre;
use matrix_sdk::{
    Client, Room,
    deserialized_responses::SyncOrStrippedState,
    reqwest::Url,
    ruma::events::{
        SyncStateEvent,
        call::member::{
            ActiveFocus, ActiveLivekitFocus, Application, CallApplicationContent,
            CallMemberEventContent, CallMemberStateKey, CallScope, Focus, LivekitFocus,
            MembershipData, OriginalSyncCallMemberEvent, SessionMembershipData,
        },
    },
};
use tokio::sync::mpsc::{Sender, channel};
use tracing::{info, warn};

use crate::matrix::{
    custom_events::EncryptionKeysChangedEvent,
    helpers::{PreferedFocus, get_livekit_token, get_openid_token, get_prefered_foci},
    livekit_session::{LiveKitSession, LiveKitTaskMessages},
};

pub struct MatrixRtcSession {
    room: Room,
    members: BTreeMap<CallMemberStateKey, SessionMembershipData>,
    sender: Sender<LiveKitTaskMessages>,
}

fn to_membership(
    membership_event: &OriginalSyncCallMemberEvent,
) -> Option<(CallMemberStateKey, SessionMembershipData)> {
    let memberships = membership_event
        .content
        .active_memberships(Some(membership_event.origin_server_ts));
    let membership = memberships.first()?;
    match membership {
        MembershipData::Session(session_membership_data) => Some((
            membership_event.state_key.clone(),
            (*session_membership_data).clone(),
        )),
        _ => {
            warn!(membership = ?membership, "Received unsupported membership type");
            None
        }
    }
}

impl MatrixRtcSession {
    pub fn has_other_members(&self) -> bool {
        self.members
            .iter()
            .any(|(member_state_key, _)| member_state_key.user_id() != self.room.own_user_id())
    }

    pub(crate) async fn join_session(room: Room) -> eyre::Result<Option<Self>> {
        let memberships = room
            .get_state_events_static::<CallMemberEventContent>()
            .await?
            .into_iter()
            .filter_map(|ev| match ev.deserialize().ok()? {
                SyncOrStrippedState::Sync(SyncStateEvent::Original(ev)) => to_membership(&ev),
                _ => None,
            })
            .collect();

        let (sender, receiver) = channel(5);

        let session = MatrixRtcSession {
            room,
            members: memberships,
            sender,
        };

        // TODO leave this behavior up to the application
        // Don't join empty calls
        if session.members.is_empty() {
            return Ok(None);
        }
        // Leave calls where we are the only member
        if !session.has_other_members() {
            session.leave_session().await?;
            return Ok(None);
        }

        let client = session.room.client();
        let our_prefered_foci = get_prefered_foci(&client).await?;
        let PreferedFocus::Livekit(livekit_info) = our_prefered_foci.list.first().unwrap() else {
            return Err(eyre!(
                "first focus in the preferred foci list is not a livekit SFU"
            ));
        };
        let our_livekit_service_url = Url::parse(&livekit_info.url)?;
        let our_focus = Focus::Livekit(LivekitFocus::new(
            session.room.room_id().to_string(),
            our_livekit_service_url.to_string(),
        ));
        let device_id = client
            .device_id()
            .expect("a logged in client should have a device id");
        let user_id = client
            .user_id()
            .expect("a logged in client should have a user id");
        let application =
            Application::Call(CallApplicationContent::new("".to_string(), CallScope::Room));
        let mut foci_list = if let Some((_, member)) = session
            .members
            .iter()
            .filter(|(_, mem)| !mem.foci_preferred.is_empty())
            .next_back()
        {
            member.foci_preferred.clone()
        } else {
            vec![]
        };
        foci_list.insert(0, our_focus);

        // This probably needs a different selection algorithm (for now just always take the first one).
        let Focus::Livekit(selected_focus) =
            foci_list.last().expect("At least out foci is in the list as we just added it (if it wasn't included previously)")
        else {
            return Err(eyre!("selected foci is not a livekit SFU"));
        };
        let livekit_service_url = Url::parse(&selected_focus.service_url)?;
        let token_response = get_openid_token(&client).await?;
        let livekit_token = get_livekit_token(
            &client,
            &session.room,
            token_response,
            livekit_service_url.clone(),
        )
        .await?;

        let join_event = CallMemberEventContent::new(
            application,
            device_id.into(),
            ActiveFocus::Livekit(ActiveLivekitFocus::new()),
            foci_list,
            None,
            None,
        );

        let mut livekit_session = LiveKitSession::new(receiver, livekit_token).await?;

        tokio::spawn(async move { livekit_session.run().await });

        let state_key = CallMemberStateKey::new(user_id.into(), Some(device_id.into()), true);
        session
            .room
            .send_state_event_for_key(&state_key, join_event)
            .await?;

        Ok(Some(session))
    }

    pub(crate) async fn leave_session(self) -> matrix_sdk::Result<()> {
        let _ = self
            .sender
            .send(LiveKitTaskMessages::LeaveLiveKitRoom)
            .await;
        let user_id = self
            .room
            .client()
            .user_id()
            .expect("client should be logged in")
            .to_owned();
        let device_id = self
            .room
            .client()
            .device_id()
            .expect("client should be logged in")
            .to_owned();
        let state_key = CallMemberStateKey::new(user_id, Some(device_id.into()), true);
        let leave_event = CallMemberEventContent::new_empty(None);

        self.room
            .send_state_event_for_key(&state_key, leave_event)
            .await?;
        Ok(())
    }

    pub(crate) fn on_rtc_member_event(
        &mut self,
        event: OriginalSyncCallMemberEvent,
    ) -> eyre::Result<()> {
        let user_id = event.state_key.user_id().to_owned();
        if let Some((state_key, membership)) = to_membership(&event) {
            if self.members.insert(state_key, membership).is_none() {
                info!(%user_id, room_id=%self.room.room_id(), "user joined the call");
            }
        } else if self.members.remove(&event.state_key).is_some() {
            info!(%user_id, room_id=%self.room.room_id(), "user left the call");
        }

        Ok(())
    }

    pub(crate) async fn on_rtc_encryption_key_changed(
        &mut self,
        _event: EncryptionKeysChangedEvent,
        _client: Client,
    ) {
        // TODO implement encryption
    }
}
