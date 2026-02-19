use std::{collections::BTreeMap, time::Duration};

use eyre::{Context, eyre};
use matrix_sdk::{
    Room,
    deserialized_responses::SyncOrStrippedState,
    reqwest::Url,
    ruma::{
        events::{
            SyncStateEvent, ToDeviceEventContent,
            call::member::{
                ActiveFocus, ActiveLivekitFocus, Application, CallApplicationContent,
                CallMemberEventContent, CallMemberStateKey, CallScope, Focus, LivekitFocus,
                MembershipData, OriginalSyncCallMemberEvent, SessionMembershipData,
            },
        },
        serde::{Base64, Raw},
    },
};
use matrix_sdk_crypto::CollectStrategy;
use rand::{TryRng, rngs::SysRng};
use tokio::{
    sync::mpsc::{Sender, channel},
    time::sleep,
};
use tracing::{info, warn};

use crate::matrix::{
    custom_events::{EncryptionKeysChangedEvent, EncryptionKeysChangedEventContent, Key, Member},
    helpers::{
        MatrixToLivekitMembership, PreferedFocus, get_livekit_token, get_openid_token,
        get_prefered_foci,
    },
    livekit_session::{LiveKitSession, LiveKitTaskMessages},
};

pub struct MatrixRtcSession {
    room: Room,
    members: BTreeMap<CallMemberStateKey, SessionMembershipData>,
    sender: Sender<LiveKitTaskMessages>,
    last_key_index: u8,
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

        let client = room.client();

        let device_id = client
            .device_id()
            .expect("a logged in client should have a device id");
        let (sender, receiver) = channel(20);

        let mut session = MatrixRtcSession {
            room,
            members: memberships,
            sender,
            last_key_index: 0_u8.wrapping_sub(1),
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
            Application::Call(CallApplicationContent::new("".to_string(), CallScope::Room)),
            device_id.into(),
            ActiveFocus::Livekit(ActiveLivekitFocus::new()),
            foci_list,
            None,
            None,
        );
        session
            .room
            .send_state_event_for_key(&session.our_state_key(), join_event)
            .await?;
        let encryption_key = if session.room.encryption_state().is_encrypted() {
            Some(session.generate_new_key(false).await?)
        } else {
            None
        };

        let mut livekit_session =
            LiveKitSession::new(receiver, livekit_token, encryption_key.is_some()).await?;

        tokio::spawn(async move { livekit_session.run().await });
        Ok(Some(session))
    }

    pub(crate) async fn leave_session(self) -> matrix_sdk::Result<()> {
        let _ = self
            .sender
            .send(LiveKitTaskMessages::LeaveLiveKitRoom)
            .await;
        let leave_event = CallMemberEventContent::new_empty(None);

        self.room
            .send_state_event_for_key(&self.our_state_key(), leave_event)
            .await?;
        Ok(())
    }

    pub(crate) async fn on_rtc_member_event(
        &mut self,
        event: OriginalSyncCallMemberEvent,
    ) -> eyre::Result<()> {
        let user_id = event.state_key.user_id().to_owned();
        let device_id = if let Some((state_key, membership)) = to_membership(&event) {
            if self.members.insert(state_key, membership.clone()).is_none() {
                info!(%user_id, room_id=%self.room.room_id(), "user joined the call");
                Some(membership.device_id.clone())
            } else {
                // User just changed membership, no key rotation needed
                return Ok(());
            }
        } else if self.members.remove(&event.state_key).is_some() {
            info!(%user_id, room_id=%self.room.room_id(), "user left the call");
            None
        } else {
            // Empty membership was resent, no need to rotate keys
            return Ok(());
        };

        let our_device_id = self
            .room
            .client()
            .device_id()
            .expect("client should be logged in")
            .to_owned();

        // Regenerate keys only in encrypted rooms, where the sender was not us
        if self.room.encryption_state().is_encrypted()
            && event.sender != self.room.own_user_id()
            && device_id != Some(our_device_id)
        {
            // TODO rate limit this (as sending keys can be expensive)
            self.generate_new_key(true).await?;
        }
        Ok(())
    }

    async fn generate_new_key(&mut self, delay_using: bool) -> eyre::Result<()> {
        // Increment key index
        let key_index = self.last_key_index.wrapping_add(1);
        self.last_key_index = key_index;

        let mut key = vec![0u8; 16];
        SysRng
            .try_fill_bytes(&mut key)
            .context("Failed to generate MatrixRTC key")?;

        let key_base64 = Base64::new(key.clone());
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

        let key = Key {
            index: key_index.into(),
            content: key_base64,
        };

        let key_changed_event = EncryptionKeysChangedEventContent {
            member: Member {
                claimed_device_id: device_id.clone(),
            },
            key: (&key).clone(),
            application: Application::Call(CallApplicationContent::new(
                "".to_string(),
                CallScope::Room,
            )),
            room_id: self.room.room_id().into(),
        };

        let mut devices = vec![];
        for (state_key, data) in &self.members {
            if state_key == &self.our_state_key() {
                // Skip own device
                continue;
            }
            if let Some(device) = self
                .room
                .client()
                .encryption()
                .get_device(state_key.user_id(), &data.device_id)
                .await?
            {
                devices.push(device);
            }
        }

        self.room
            .client()
            .encryption()
            .encrypt_and_send_raw_to_device(
                devices.iter().collect(),
                &key_changed_event.event_type().to_string(),
                Raw::new(&key_changed_event)?.cast(),
                CollectStrategy::IdentityBasedStrategy,
            )
            .await?;

        if delay_using {
            tokio::spawn({
                let sender = self.sender.clone();
                async move {
                    // According to MSC4143 the default delay before using a new key is 5 seconds
                    sleep(Duration::from_secs(5)).await;

                    let _ = sender
                        .send(LiveKitTaskMessages::KeyChanged(
                            MatrixToLivekitMembership::new(user_id, device_id).into(),
                            key.clone(),
                        ))
                        .await
                        .inspect_err(|err| warn!(error = %err, "Sending keys to LiveKit failed"));
                }
            });
        } else {
            self.sender
                .send(LiveKitTaskMessages::KeyChanged(
                    MatrixToLivekitMembership::new(user_id, device_id).into(),
                    key.clone(),
                ))
                .await
                .context("Failed to send key to LiveKit task")?;
        }

        Ok(())
    }

    pub(crate) async fn on_rtc_encryption_key_changed(
        &mut self,
        event: EncryptionKeysChangedEvent,
    ) -> eyre::Result<()> {
        let room_id = event.content.room_id;
        let user_id = event.sender;
        let device_id = event.content.member.claimed_device_id;
        let livekit_identity = MatrixToLivekitMembership::new(user_id, device_id);
        info!(
            "in room {}, livekit member {} has key {}",
            room_id,
            livekit_identity.to_string(),
            event.content.key.content
        );
        if self.room.encryption_state().is_encrypted() {
            // TODO ignore keys that were shared through an unencrypted ToDevice event (if possible)

            self.sender
                .send(LiveKitTaskMessages::KeyChanged(
                    livekit_identity.into(),
                    event.content.key,
                ))
                .await?;
        } else {
            warn!("Received key for call in unencrypted room")
        }
        Ok(())
    }
    fn our_state_key(&self) -> CallMemberStateKey {
        let client = self.room.client();
        let user_id = client
            .user_id()
            .expect("client should be logged in")
            .to_owned();

        let device_id = client.device_id().expect("client should be logged in");
        CallMemberStateKey::new(user_id, Some(device_id.to_string()), true)
    }
}
