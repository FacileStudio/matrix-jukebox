use std::{collections::HashMap, sync::Arc};

use matrix_sdk::{
    Client, Room, RoomState,
    ruma::{OwnedRoomId, events::call::member::OriginalSyncCallMemberEvent},
};
use tokio::sync::Mutex;
use tracing::{info, instrument, trace, warn};

use crate::matrix::{custom_events::EncryptionKeysChangedEvent, rtc_session::MatrixRtcSession};

#[derive(Clone)]
pub struct MatrixRtcSessionManager {
    inner: Arc<Mutex<MatrixRtcSessionManagerInner>>,
}

impl MatrixRtcSessionManager {
    pub async fn init(client: &Client) -> eyre::Result<Self> {
        // TODO: Should we make sure that this is the only instance (at least for the client)?
        let mut sessions = HashMap::new();
        for room in client
            .rooms()
            .into_iter()
            .filter(|r| r.state() == RoomState::Joined)
        {
            let Some(session) = MatrixRtcSession::join_session(room.clone()).await? else {
                continue;
            };
            sessions.insert(room.room_id().to_owned(), session);
        }
        let inner = MatrixRtcSessionManagerInner { sessions };

        let manager = Self {
            inner: Arc::new(Mutex::new(inner)),
        };
        {
            let manager = manager.clone();

            client.add_event_handler(
                |event: OriginalSyncCallMemberEvent, room: Room| async move {
                    manager.on_rtc_member_changed(event, room).await
                },
            );
        }
        {
            let manager = manager.clone();
            client.add_event_handler(
                |event: EncryptionKeysChangedEvent, client: Client| async move {
                    manager.on_rtc_encryption_key_changed(event, client).await
                },
            );
        }

        Ok(manager)
    }
    async fn on_rtc_member_changed(
        &self,
        event: OriginalSyncCallMemberEvent,
        room: Room,
    ) -> eyre::Result<()> {
        self.inner
            .lock()
            .await
            .on_rtc_member_changed(event, room)
            .await
    }
    async fn on_rtc_encryption_key_changed(
        &self,
        event: EncryptionKeysChangedEvent,
        client: Client,
    ) {
        self.inner
            .lock()
            .await
            .on_rtc_encryption_key_changed(event, client)
            .await
    }
}

struct MatrixRtcSessionManagerInner {
    sessions: HashMap<OwnedRoomId, MatrixRtcSession>,
}

impl MatrixRtcSessionManagerInner {
    #[instrument(skip(self, event, room), fields(room = %room.room_id(), sender = %event.sender))]
    async fn on_rtc_member_changed(
        &mut self,
        event: OriginalSyncCallMemberEvent,
        room: Room,
    ) -> eyre::Result<()> {
        trace!(?event, "received event!");
        let room_id = room.room_id();
        if let Some(session) = self.sessions.get_mut(room_id) {
            session.on_rtc_member_event(event)?;

            if !session.has_other_members() {
                self.sessions
                    .remove(room_id)
                    .expect("We just used this session")
                    .leave_session()
                    .await?;
            }
        } else {
            // Don't create calls for our own events (Should we also check device ID here?)
            if event.sender != room.own_user_id() {
                // Create a call for this session
                let room_id = room.room_id().to_owned();
                let Some(session) = MatrixRtcSession::join_session(room).await? else {
                    return Ok(());
                };

                self.sessions.insert(room_id, session);
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn on_rtc_encryption_key_changed(
        &mut self,
        event: EncryptionKeysChangedEvent,
        client: Client,
    ) {
        info!(?event, "received event!");
        let room_id = &event.content.room_id;
        if let Some(session) = self.sessions.get_mut(room_id) {
            session.on_rtc_encryption_key_changed(event, client).await;
        } else {
            warn!(?event, "received key change for unknown call!");
        }
    }
}
