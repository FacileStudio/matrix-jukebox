use livekit::{
    Room, RoomOptions,
    options::TrackPublishOptions,
    track::{LocalAudioTrack, LocalTrack, TrackSource},
    webrtc::{
        audio_source::native::NativeAudioSource,
        prelude::{AudioFrame, AudioSourceOptions, RtcAudioSource},
    },
};
use rodio::{
    ChannelCount, SampleRate, conversions::SampleTypeConverter, mixer::mixer, source::noise,
};

use tokio::sync::mpsc::{Receiver, error::TryRecvError};
use tracing::{info, instrument};

use super::helpers::LivekitTokenResponse;

pub enum LiveKitTaskMessages {
    LeaveLiveKitRoom,
}

#[derive(Debug)]
pub struct LiveKitSession {
    receiver: Receiver<LiveKitTaskMessages>,
    room: Room,
}

impl LiveKitSession {
    pub async fn new(
        receiver: Receiver<LiveKitTaskMessages>,
        token_rest: LivekitTokenResponse,
    ) -> eyre::Result<Self> {
        let options = RoomOptions::default();
        let (room, _room_events) =
            Room::connect(token_rest.url.as_str(), &token_rest.token, options).await?;

        Ok(Self { receiver, room })
    }
    #[instrument]
    pub async fn run(&mut self) -> eyre::Result<()> {
        const SAMPLE_RATE: SampleRate = 48000;
        const CHANNEL_COUNT: ChannelCount = 2;

        let (mixer, mixer_source) = mixer(CHANNEL_COUNT, SAMPLE_RATE);
        let mut mixer_source_converted = SampleTypeConverter::new(mixer_source);

        // TODO give the mixer to some playback handler instead of noise
        mixer.add(noise::Pink::new(SAMPLE_RATE));

        let source = NativeAudioSource::new(
            AudioSourceOptions::default(),
            SAMPLE_RATE,
            CHANNEL_COUNT.into(),
            1000, // Buffer 1 second
        );

        let track: LocalAudioTrack =
            LocalAudioTrack::create_audio_track("file", RtcAudioSource::Native(source.clone()));

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

        const CHUNK_SIZE: u32 = SAMPLE_RATE; // Buffer 1s

        let mut audio_frame = AudioFrame::new(SAMPLE_RATE, CHANNEL_COUNT.into(), CHUNK_SIZE);

        loop {
            match self.receiver.try_recv() {
                Ok(message) => match message {
                    LiveKitTaskMessages::LeaveLiveKitRoom => {
                        break;
                    }
                },
                Err(TryRecvError::Empty) => {}
                Err(err) => return Err(err.into()),
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
