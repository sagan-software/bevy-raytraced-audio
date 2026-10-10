//! Processed decoder for Bevy releases built on rodio 0.20.

use crate::processed_audio::{ProcessedSamples, RaytracedAudioSource};
use bevy::audio::{AudioSource, Decodable, Sample, Source};
use core::time::Duration;

/// The decoder type Bevy uses for encoded audio assets.
type InnerDecoder = <AudioSource as Decodable>::Decoder;

/// Converts Bevy's integer decoder output to `f32` samples.
pub struct FloatSamples(InnerDecoder);

impl core::fmt::Debug for FloatSamples {
    /// Formats the wrapper; decoder internals are opaque.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FloatSamples")
            .finish_non_exhaustive()
    }
}

impl Iterator for FloatSamples {
    type Item = f32;

    /// Returns the next decoded sample as `f32`.
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Sample::to_f32)
    }
}

/// Decodes one processed source for Bevy's audio sink.
pub struct ProcessedDecoder {
    /// Shared looping, processing, and tail state.
    samples: ProcessedSamples<FloatSamples>,
    /// Interleaved channel count of the source.
    channels: u16,
    /// Sample rate of the source in hertz.
    sample_rate: u32,
}

/// Creates a fresh decoder for one loop repetition.
fn restart(source: &AudioSource) -> FloatSamples {
    FloatSamples(source.decoder())
}

impl core::fmt::Debug for ProcessedDecoder {
    /// Formats the stream layout; decoder internals are opaque.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ProcessedDecoder")
            .field("channels", &self.channels)
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

impl Iterator for ProcessedDecoder {
    type Item = f32;

    /// Returns the next processed interleaved sample.
    fn next(&mut self) -> Option<Self::Item> {
        self.samples.next_sample()
    }
}

impl Source for ProcessedDecoder {
    /// Reports constant parameters for the whole stream.
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    /// Returns the source channel count.
    fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the source sample rate in hertz.
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Reports an unknown duration because loops and reverb tails extend playback.
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for RaytracedAudioSource {
    type DecoderItem = f32;
    type Decoder = ProcessedDecoder;

    /// Decodes the wrapped asset through this entity's acoustic processor.
    fn decoder(&self) -> Self::Decoder {
        let inner = self.source.decoder();
        let channels = inner.channels();
        let sample_rate = inner.sample_rate();
        ProcessedDecoder {
            samples: ProcessedSamples::new(
                FloatSamples(inner),
                restart,
                self,
                channels,
                sample_rate,
            ),
            channels: if self.binaural.is_some() { 2 } else { channels },
            sample_rate,
        }
    }
}
