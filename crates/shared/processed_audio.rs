//! Opt-in sample processing that makes traced muffling and reverb audible.
//!
//! Bevy's built-in sinks expose volume, speed, and panning but no filter insertion. A
//! [`RaytracedAudioPlayer`] therefore wraps a loaded [`AudioSource`] in a per-entity
//! [`RaytracedAudioSource`] asset whose decoder runs an [`AcousticDspProcessor`]. The 2D and 3D
//! adapters write each emitter's traced filter and the listener's reverb into the shared
//! [`AcousticDspParams`]; the audio thread reads them without locks.

use bevy::asset::{Asset, Assets, Handle};
use bevy::audio::{AudioPlayer, AudioSource, PlaybackMode, PlaybackSettings};
use bevy::prelude::{App, Commands, Component, Entity, Plugin, Query, Res, ResMut, Without};
use bevy::reflect::TypePath;
use bevy_raytraced_audio::{
    AcousticDspParams, AcousticDspProcessor, BinauralParams, BinauralProcessor,
};
use std::sync::Arc;

/// Plays a Bevy audio asset through traced muffling and reverb.
///
/// Spawn it instead of [`AudioPlayer`] on an entity that also carries a 2D or 3D emitter marker
/// and [`PlaybackSettings`]. `PlaybackSettings::LOOP` is supported; the processed source loops
/// internally so its filter keeps following the scene on every repetition.
#[derive(Component, Clone, Debug)]
pub struct RaytracedAudioPlayer {
    /// The decoded audio asset to play.
    source: Handle<AudioSource>,
    /// Parameters shared with the audio thread.
    params: Arc<AcousticDspParams>,
    /// Scale applied to the listener's traced reverb send.
    reverb_send: f32,
    /// Optional point-source headphone renderer; plain and 2D sources retain their layout.
    binaural: Option<Arc<BinauralParams>>,
}

impl RaytracedAudioPlayer {
    /// Creates a processed player for one audio asset.
    #[must_use]
    pub fn new(source: Handle<AudioSource>) -> Self {
        Self {
            source,
            params: Arc::new(AcousticDspParams::default()),
            reverb_send: 1.0,
            binaural: None,
        }
    }

    /// Enables measured 3D headphone rendering with inverse-distance rolloff.
    ///
    /// The 3D adapter supplies listener-local XYZ, including head rotation. This player
    /// uses a regular stereo sink; Bevy's spatial panner is disabled to avoid downmixing
    /// the binaural signal. `distance_scale` replaces Bevy's spatial scale for this voice.
    /// Stereo recordings are downmixed to a mono point source before rendering.
    #[must_use]
    pub fn with_binaural(mut self, distance_scale: f32) -> Self {
        self.binaural = Some(Arc::new(BinauralParams::new(distance_scale)));
        self
    }

    /// Returns headphone controls, if this player was configured with [`Self::with_binaural`].
    #[must_use]
    pub const fn binaural_params(&self) -> Option<&Arc<BinauralParams>> {
        self.binaural.as_ref()
    }

    /// Scales the traced reverb send for this sound; `0` keeps it dry.
    #[must_use]
    pub const fn with_reverb_send(mut self, send: f32) -> Self {
        self.reverb_send = if send.is_finite() {
            send.clamp(0.0, 2.0)
        } else {
            1.0
        };
        self
    }

    /// Returns the played audio asset.
    #[must_use]
    pub const fn source(&self) -> &Handle<AudioSource> {
        &self.source
    }

    /// Returns the parameters shared with the audio thread.
    #[must_use]
    pub const fn params(&self) -> &Arc<AcousticDspParams> {
        &self.params
    }

    /// Returns the reverb send scale.
    #[must_use]
    pub const fn reverb_send(&self) -> f32 {
        self.reverb_send
    }

    /// Enables processing, or bypasses it for an A/B comparison.
    pub fn set_processing_enabled(&self, enabled: bool) {
        self.params.set_enabled(enabled);
    }
}

/// A per-entity audio asset that decodes a source through acoustic processing.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct RaytracedAudioSource {
    /// Encoded audio shared with the original asset.
    pub(crate) source: AudioSource,
    /// Parameters shared with the game thread.
    pub(crate) params: Arc<AcousticDspParams>,
    /// Whether the decoder restarts the source internally instead of ending.
    pub(crate) looping: bool,
    /// Optional listener-relative headphone controls.
    pub(crate) binaural: Option<Arc<BinauralParams>>,
}

/// System set that turns loaded [`RaytracedAudioPlayer`] sources into Bevy audio players.
///
/// Adapters trace before this set so new sounds start with current muffling and reverb.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RaytracedAudioPrepareSystems;

/// Marks a processed player whose asset has been created.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaytracedAudioPrepared;

/// Registers the processed audio asset and the system that creates per-entity sources.
#[derive(Clone, Copy, Debug, Default)]
pub struct RaytracedAudioProcessingPlugin;

impl Plugin for RaytracedAudioProcessingPlugin {
    /// Adds the processed source type to Bevy's audio playback systems.
    fn build(&self, app: &mut App) {
        use bevy::audio::AddAudioSource;
        use bevy::ecs::schedule::IntoScheduleConfigs;
        use bevy::prelude::{PostUpdate, TransformSystems};

        app.add_audio_source::<RaytracedAudioSource>().add_systems(
            PostUpdate,
            prepare_raytraced_players
                .in_set(RaytracedAudioPrepareSystems)
                .after(TransformSystems::Propagate),
        );
    }
}

/// Wraps each loaded player source in its own processed asset and queues Bevy playback.
pub(crate) fn prepare_raytraced_players(
    mut commands: Commands<'_, '_>,
    sources: Res<'_, Assets<AudioSource>>,
    mut processed: ResMut<'_, Assets<RaytracedAudioSource>>,
    mut players: Query<
        '_,
        '_,
        (Entity, &RaytracedAudioPlayer, Option<&mut PlaybackSettings>),
        Without<RaytracedAudioPrepared>,
    >,
) {
    for (entity, player, mut settings) in &mut players {
        let Some(source) = sources.get(&player.source) else {
            continue;
        };
        if player.binaural.is_some() {
            if let Some(settings) = settings.as_deref_mut() {
                settings.spatial = false;
            } else {
                commands
                    .entity(entity)
                    .insert(PlaybackSettings::default().with_spatial(false));
            }
        }
        // Bevy loops by buffering decoded samples, which would freeze the filter after one pass.
        let looping = if let Some(mut settings) = settings
            && matches!(settings.mode, PlaybackMode::Loop)
        {
            settings.mode = PlaybackMode::Once;
            true
        } else {
            false
        };
        let handle = processed.add(RaytracedAudioSource {
            source: source.clone(),
            params: Arc::clone(&player.params),
            looping,
            binaural: player.binaural.clone(),
        });
        commands
            .entity(entity)
            .insert((AudioPlayer(handle), RaytracedAudioPrepared));
    }
}

/// Silence required before ending a tail, in seconds.
///
/// This exceeds the DSP's 300 ms pre-delay plus its comb and all-pass delays, so a
/// quiet gap before the first echo cannot end playback before that echo arrives.
const TAIL_SILENCE_S: f32 = 0.5;

/// Absolute level below which a tail sample counts as silent.
const TAIL_SILENCE_LEVEL: f32 = 1.0e-4;

/// Longest reverb tail rendered after a non-looping source ends, in seconds.
const MAXIMUM_TAIL_S: f32 = 6.0;

/// Converts a bounded tail duration to frames for one sample rate.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a few seconds of audio frames fit exactly in both f32 and usize"
)]
fn duration_frames(duration_s: f32, sample_rate_hz: u32) -> usize {
    (duration_s * sample_rate_hz as f32) as usize
}

/// Version-independent decoding state shared by the rodio adapters.
///
/// `D` yields interleaved `f32` samples. Looping sources restart `D` with `restart` when it ends;
/// one-shot sources keep producing processor output after the input ends so the reverb tail rings
/// out instead of being cut off.
pub(crate) struct ProcessedSamples<D> {
    /// Current decoder of the encoded source.
    inner: D,
    /// Creates a fresh decoder for looping playback.
    restart: fn(&AudioSource) -> D,
    /// Shared acoustic processor.
    processor: AcousticDspProcessor,
    /// Optional mono-to-stereo headphone renderer.
    binaural: Option<BinauralProcessor>,
    /// Decoded input channels, which may differ from the two output channels.
    input_channels: usize,
    /// Right-ear sample waiting for the next iterator call.
    pending_right: Option<f32>,
    /// Encoded source used to restart looping playback.
    asset: RaytracedAudioSource,
    /// Number of interleaved channels.
    channels: usize,
    /// Interleaved samples emitted so far, modulo the channel count.
    channel_position: usize,
    /// Maximum tail length in interleaved samples.
    tail_limit: usize,
    /// Remaining tail samples after the source ended, or `None` while the source plays.
    tail_remaining: Option<usize>,
    /// Consecutive near-silent tail samples.
    silent_samples: usize,
    /// Silence window in interleaved samples, scaled to the stream's sample rate.
    silence_limit: usize,
}

impl<D: Iterator<Item = f32>> ProcessedSamples<D> {
    /// Wraps a decoder with a processor for its channel layout and sample rate.
    pub(crate) fn new(
        inner: D,
        restart: fn(&AudioSource) -> D,
        asset: &RaytracedAudioSource,
        channels: u16,
        sample_rate_hz: u32,
    ) -> Self {
        let input_channels = usize::from(channels.max(1));
        let channel_count = if asset.binaural.is_some() {
            2
        } else {
            input_channels
        };
        Self {
            inner,
            restart,
            processor: AcousticDspProcessor::new(
                Arc::clone(&asset.params),
                if asset.binaural.is_some() {
                    1
                } else {
                    channels
                },
                sample_rate_hz,
            ),
            binaural: asset
                .binaural
                .as_ref()
                .map(|params| BinauralProcessor::new(Arc::clone(params), sample_rate_hz)),
            input_channels,
            pending_right: None,
            asset: asset.clone(),
            channels: channel_count,
            channel_position: 0,
            tail_limit: duration_frames(MAXIMUM_TAIL_S, sample_rate_hz) * channel_count,
            tail_remaining: None,
            silent_samples: 0,
            silence_limit: duration_frames(TAIL_SILENCE_S, sample_rate_hz) * channel_count,
        }
    }

    /// Returns the next processed interleaved sample, or `None` after the tail ends.
    pub(crate) fn next_sample(&mut self) -> Option<f32> {
        if self.binaural.is_some() {
            return self.next_binaural_sample();
        }
        let input = match self.tail_remaining {
            Some(_) => None,
            None => self.next_input(),
        };
        let sample = match input {
            Some(input) => self.processor.process_sample(input),
            None => self.next_tail_sample()?,
        };
        self.channel_position += 1;
        if self.channel_position >= self.channels {
            self.channel_position = 0;
        }
        Some(sample)
    }

    /// Downmixes each input frame once, processes it, and emits a complete stereo frame.
    #[expect(
        clippy::cast_precision_loss,
        reason = "audio channel counts are small integers"
    )]
    fn next_binaural_sample(&mut self) -> Option<f32> {
        if let Some(right) = self.pending_right.take() {
            return Some(right);
        }
        if self
            .tail_remaining
            .is_some_and(|remaining| remaining == 0 || self.silent_samples >= self.silence_limit)
        {
            return None;
        }
        let mut mono = 0.0;
        let mut had_input = false;
        if self.tail_remaining.is_none() {
            for _ in 0..self.input_channels {
                if let Some(sample) = self.next_input() {
                    mono += sample;
                    had_input = true;
                }
            }
            mono /= self.input_channels as f32;
        }
        if !had_input {
            let remaining = self.tail_remaining.get_or_insert(self.tail_limit);
            *remaining = remaining.saturating_sub(2);
        }
        let (direct, diffuse) = self.processor.process_sample_split(mono);
        let [left, right] = self.binaural.as_mut()?.process_frame(direct, diffuse);
        if !had_input {
            if left.abs().max(right.abs()) < TAIL_SILENCE_LEVEL {
                self.silent_samples += 2;
            } else {
                self.silent_samples = 0;
            }
        }
        self.pending_right = Some(right);
        Some(left)
    }

    /// Reads the next input sample, restarting looping sources once.
    fn next_input(&mut self) -> Option<f32> {
        if let Some(sample) = self.inner.next() {
            return Some(sample);
        }
        if self.asset.looping {
            self.inner = (self.restart)(&self.asset.source);
            // An empty source would otherwise restart forever.
            return self.inner.next();
        }
        None
    }

    /// Produces one reverb-tail sample, ending on a frame boundary once the tail is silent.
    fn next_tail_sample(&mut self) -> Option<f32> {
        let remaining = self.tail_remaining.get_or_insert(self.tail_limit);
        let tail_is_done = *remaining == 0 || self.silent_samples >= self.silence_limit;
        if tail_is_done && self.channel_position == 0 {
            return None;
        }
        *remaining = remaining.saturating_sub(1);
        let sample = self.processor.process_sample(0.0);
        if sample.abs() < TAIL_SILENCE_LEVEL {
            self.silent_samples += 1;
        } else {
            self.silent_samples = 0;
        }
        Some(sample)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/processed_audio.rs"]
mod tests;
