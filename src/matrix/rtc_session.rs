use std::collections::HashMap;

use eyre::eyre;
use matrix_sdk::{
    Client, Room,
    deserialized_responses::SyncOrStrippedState,
    reqwest::Url,
    ruma::{
        OwnedUserId, UserId,
        events::{
            SyncStateEvent,
            call::member::{
                ActiveFocus, ActiveLivekitFocus, Application, CallApplicationContent,
                CallMemberEventContent, CallMemberStateKey, CallScope, Focus, LivekitFocus,
                MembershipData, OriginalSyncCallMemberEvent, SessionMembershipData,
            },
        },
    },
};
use tokio::sync::mpsc::{Sender, channel};
use tracing::info;

use crate::matrix::{
    custom_events::EncryptionKeysChangedEvent,
    helpers::{PreferedFocus, get_livekit_token, get_openid_token, get_prefered_foci},
    livekit_session::{LiveKitSession, LiveKitTaskMessages},
};

pub struct MatrixRtcSession {
    room: Room,
    members: HashMap<OwnedUserId, SessionMembershipData>,
    sender: Sender<LiveKitTaskMessages>,
}

fn to_membership(
    membership_data: MembershipData,
    sender: &UserId,
) -> eyre::Result<(OwnedUserId, SessionMembershipData)> {
    match membership_data {
        MembershipData::Legacy(_) => Err(eyre!("we don't support legacy matrix rtc sessions")),
        MembershipData::Session(session_membership_data) => {
            Ok((sender.to_owned(), session_membership_data.clone()))
        }
        _ => Err(eyre!("unsupported matrix rtc session type")),
    }
}

impl MatrixRtcSession {
    pub fn has_other_members(&self) -> bool {
        self.members
            .iter()
            .any(|(member, _)| member != self.room.own_user_id())
    }

    pub(crate) async fn join_session(room: Room) -> eyre::Result<Option<Self>> {
        let memberships = room
            .get_state_events_static::<CallMemberEventContent>()
            .await?
            .into_iter()
            .filter_map(|ev| match ev.deserialize().ok()? {
                SyncOrStrippedState::Sync(SyncStateEvent::Original(ev)) => ev
                    .content
                    .memberships()
                    .get(0)
                    .map(|md| to_membership(md.clone(), &ev.sender).ok())
                    .flatten(),
                _ => None,
            })
            .collect();

        let user_id = room
            .client()
            .user_id()
            .expect("client should be logged in")
            .to_owned();

        let (sender, receiver) = channel(5);

        let session = MatrixRtcSession {
            room,
            members: memberships,
            sender,
        };
        // Don't join empty calls
        if session.members.len() == 0 {
            return Ok(None);
        }
        // Leave calls where we are the only member
        if session.members.len() == 1 && session.members.contains_key(&user_id) {
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
            .filter(|(_, mem)| mem.foci_preferred.len() != 0)
            .last()
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
        member: OriginalSyncCallMemberEvent,
    ) -> eyre::Result<()> {
        let user_id = &member.sender;
        if let Some(membership) = member
            .content
            .active_memberships(Some(member.origin_server_ts))
            .into_iter()
            .filter_map(|membership| {
                if let MembershipData::Session(membership) = membership {
                    Some(membership)
                } else {
                    None
                }
            })
            .next()
        {
            info!(%user_id, room_id=%self.room.room_id(), "joined the call");
            self.members.insert(user_id.to_owned(), membership.clone());
        } else {
            info!(%user_id, room_id=%self.room.room_id(), "left the call");
            self.members.remove(user_id);
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
