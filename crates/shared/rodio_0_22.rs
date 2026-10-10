//! Processed decoder for Bevy releases built on rodio 0.22.

use crate::processed_audio::{ProcessedSamples, RaytracedAudioSource};
use bevy::audio::{AudioSource, ChannelCount, Decodable, Sample, SampleRate, Source};
use core::time::Duration;

/// The decoder type Bevy uses for encoded audio assets.
type InnerDecoder = <AudioSource as Decodable>::Decoder;

/// Decodes one processed source for Bevy's audio sink.
pub struct ProcessedDecoder {
    /// Shared looping, processing, and tail state.
    samples: ProcessedSamples<InnerDecoder>,
    /// Interleaved channel count of the source.
    channels: ChannelCount,
    /// Sample rate of the source.
    sample_rate: SampleRate,
}

/// Creates a fresh decoder for one loop repetition.
fn restart(source: &AudioSource) -> InnerDecoder {
    source.decoder()
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
    type Item = Sample;

    /// Returns the next processed interleaved sample.
    fn next(&mut self) -> Option<Self::Item> {
        self.samples.next_sample()
    }
}

impl Source for ProcessedDecoder {
    /// Reports constant parameters for the whole stream.
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    /// Returns the source channel count.
    fn channels(&self) -> ChannelCount {
        self.channels
    }

    /// Returns the source sample rate.
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// Reports an unknown duration because loops and reverb tails extend playback.
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for RaytracedAudioSource {
    type Decoder = ProcessedDecoder;

    /// Decodes the wrapped asset through this entity's acoustic processor.
    fn decoder(&self) -> Self::Decoder {
        let inner = self.source.decoder();
        let channels = inner.channels();
        let sample_rate = inner.sample_rate();
        ProcessedDecoder {
            samples: ProcessedSamples::new(inner, restart, self, channels.get(), sample_rate.get()),
            channels: if self.binaural.is_some() {
                ChannelCount::new(2).expect("stereo")
            } else {
                channels
            },
            sample_rate,
        }
    }
}
