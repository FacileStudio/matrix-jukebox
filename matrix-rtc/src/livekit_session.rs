use livekit::{
    Room, RoomOptions,
    e2ee::key_provider::{KeyDerivationAlgorithm, KeyProvider, KeyProviderOptions},
    id::ParticipantIdentity,
    options::TrackPublishOptions,
    track::{LocalAudioTrack, LocalTrack, TrackSource},
    webrtc::{
        audio_source::native::NativeAudioSource,
        prelude::{AudioFrame, AudioSourceOptions, RtcAudioSource},
    },
};
use rodio::{conversions::SampleTypeConverter, mixer::mixer, source::noise};

use tokio::sync::mpsc::{Receiver, error::TryRecvError};
use tracing::{info, instrument};

use crate::custom_events::Key;

use super::helpers::LivekitTokenResponse;

pub enum LiveKitTaskMessages {
    LeaveLiveKitRoom,
    KeyChanged(ParticipantIdentity, Key),
}

pub struct LiveKitSession {
    receiver: Receiver<LiveKitTaskMessages>,
    key_provider: KeyProvider,
    room: Room,
}

impl LiveKitSession {
    pub async fn new(
        receiver: Receiver<LiveKitTaskMessages>,
        token_rest: LivekitTokenResponse,
        enable_encryption: bool,
    ) -> eyre::Result<Self> {
        let mut options = RoomOptions::default();
        let mut key_provider_options = KeyProviderOptions::default();
        key_provider_options.ratchet_window_size = 10;
        key_provider_options.key_ring_size = 256;
        key_provider_options.key_derivation_algorithm = KeyDerivationAlgorithm::HKDF;

        let key_provider = KeyProvider::new(key_provider_options);
        options.encryption = enable_encryption.then(|| livekit::E2eeOptions {
            encryption_type: livekit::e2ee::EncryptionType::Gcm,
            key_provider: key_provider.clone(),
        });

        let (room, _room_events) =
            Room::connect(token_rest.url.as_str(), &token_rest.token, options).await?;

        if enable_encryption {
            room.e2ee_manager().set_enabled(true);
        }
        Ok(Self {
            receiver,
            key_provider,
            room,
        })
    }
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> eyre::Result<()> {
        const SAMPLE_RATE: u32 = 48000;
        const CHANNEL_COUNT: u16 = 2;

        let (mixer, mixer_source) = mixer(
            CHANNEL_COUNT.try_into().expect("Constant is not zero"),
            SAMPLE_RATE.try_into().expect("Constant is not zero"),
        );
        let mut mixer_source_converted = SampleTypeConverter::new(mixer_source);

        // TODO give the mixer to some playback handler instead of noise
        mixer.add(noise::Pink::new(
            SAMPLE_RATE.try_into().expect("Constant is not zero"),
        ));

        let source = NativeAudioSource::new(
            AudioSourceOptions::default(),
            SAMPLE_RATE,
            CHANNEL_COUNT.into(),
            1000, // Buffer 1 second
        );

        let track: LocalAudioTrack =
            LocalAudioTrack::create_audio_track("", RtcAudioSource::Native(source.clone()));

        let _publication = self
            .room
            .local_participant()
            .publish_track(
                LocalTrack::Audio(track),
                TrackPublishOptions {
                    source: TrackSource::Microphone,
                    ..Default::default()
                },
            )
            .await?;

        let chunk_size: u32 = SAMPLE_RATE.into(); // Buffer 1s

        let mut audio_frame = AudioFrame::new(SAMPLE_RATE, CHANNEL_COUNT.into(), chunk_size);

        let mut sub = self.room.subscribe();
        loop {
            match self.receiver.try_recv() {
                Ok(LiveKitTaskMessages::LeaveLiveKitRoom) => {
                    break;
                }
                Ok(LiveKitTaskMessages::KeyChanged(participant, key)) => {
                    info!("{}: Adding new key with index {}", participant, key.index);
                    self.key_provider.set_key(
                        &participant,
                        key.index as i32,
                        key.content.into_inner(),
                    );
                }
                Err(TryRecvError::Empty) => {}
                Err(err) => return Err(err.into()),
            }
            while let Ok(msg) = sub.try_recv() {
                match msg {
                    livekit::RoomEvent::E2eeStateChanged { participant, state } => {
                        info!("{}: E2eeStateChanged: {state:?}", participant.identity())
                    }
                    livekit::RoomEvent::ParticipantEncryptionStatusChanged {
                        participant,
                        is_encrypted,
                    } => {
                        info!(
                            "{}: ParticipantEncryptionStatusChanged: {is_encrypted}",
                            participant.identity()
                        )
                    }
                    livekit::RoomEvent::ActiveSpeakersChanged { .. }
                    | livekit::RoomEvent::ConnectionQualityChanged { .. }
                    | livekit::RoomEvent::ParticipantMetadataChanged { .. }
                    | livekit::RoomEvent::ParticipantsUpdated { .. }
                    | livekit::RoomEvent::TrackMuted { .. }
                    | livekit::RoomEvent::RoomUpdated { .. }
                    | livekit::RoomEvent::TrackUnmuted { .. } => {}
                    event => {
                        info!("other LK event: {event:?}")
                    }
                }
            }
            // TODO Maybe handle empty mixer_source better? Should we mute on the livekit side?
            audio_frame
                .data
                .to_mut()
                .fill_with(|| mixer_source_converted.next().unwrap_or(0));

            source.capture_frame(&audio_frame).await.unwrap();
        }
        info!("Leaving room");
        self.room.close().await?;
        Ok(())
    }
}
