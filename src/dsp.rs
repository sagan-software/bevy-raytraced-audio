//! Real-time muffling and reverb processing driven by ray-trace results.
//!
//! [`AcousticDspParams`] is shared between the game thread and the audio thread. The game thread
//! writes the latest [`MuffleFilter`] and [`ReverbEstimate`]; [`AcousticDspProcessor`] reads them
//! on the audio thread, smooths every change, and processes interleaved samples without locking or
//! allocating.
//!
//! The filter splits the signal at a fixed crossover with two cascaded one-pole low-pass stages,
//! then scales the low and high parts by `gain_lf` and `gain_hf`. The reverb sends the filtered
//! signal through a pre-delay line and a small Schroeder network: four damped feedback combs in
//! parallel followed by two series all-pass stages, as in Freeverb. Low/high comb feedback
//! follows material-dependent decay times, and a separate early return survives outdoors.

#![expect(
    clippy::suboptimal_flops,
    reason = "audio tolerance permits separate arithmetic; generic native and wasm FMA otherwise dispatch to software per sample"
)]

use crate::{MuffleFilter, ReverbEstimate};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Crossover between the low and high bands, in hertz.
const CROSSOVER_HZ: f32 = 900.0;

/// Time constant for parameter smoothing, in seconds.
const SMOOTHING_S: f32 = 0.03;

/// Frames processed between reads of the shared parameters.
const PARAMETER_REFRESH_FRAMES: u32 = 64;

/// Longest supported pre-delay, in seconds.
const MAXIMUM_PRE_DELAY_S: f32 = 0.3;

/// Freeverb comb lengths in samples at 44.1 kHz.
const COMB_LENGTHS_44K: [usize; 4] = [1116, 1277, 1422, 1617];

/// Freeverb all-pass lengths in samples at 44.1 kHz.
const ALLPASS_LENGTHS_44K: [usize; 2] = [556, 341];

/// Feedback gain of each all-pass stage.
const ALLPASS_FEEDBACK: f32 = 0.5;

/// Overall reverb output level before the traced wet gain.
const REVERB_OUTPUT_SCALE: f32 = 0.6;

/// Converts a small sample count to `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "delay lengths and channel counts are far below f32's exact integer range"
)]
const fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Converts a sample rate in hertz to `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "audio sample rates are far below f32's exact integer range"
)]
const fn rate_to_f32(rate_hz: u32) -> f32 {
    rate_hz as f32
}

/// Converts a non-negative duration in samples to a whole sample count, rounding down.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to a finite, non-negative sample count before conversion"
)]
const fn samples_to_count(samples: f32) -> usize {
    if samples.is_finite() {
        samples.max(0.0) as usize
    } else {
        0
    }
}

/// Shared, lock-free processing parameters for one sound.
#[derive(Debug)]
pub struct AcousticDspParams {
    /// Low-frequency gain as `f32` bits.
    gain_lf: AtomicU32,
    /// High-frequency gain as `f32` bits.
    gain_hf: AtomicU32,
    /// Reverb send gain as `f32` bits.
    wet_gain: AtomicU32,
    /// 60 dB decay time in seconds as `f32` bits.
    decay_time_s: AtomicU32,
    /// Low-band decay time as float bits.
    decay_low_s: AtomicU32,
    /// High-band decay time as float bits.
    decay_high_s: AtomicU32,
    /// Early-return send gain as float bits.
    early_gain: AtomicU32,
    /// Pre-delay in seconds as `f32` bits.
    pre_delay_s: AtomicU32,
    /// Whether processing is active; disabled processing passes audio through unchanged.
    enabled: AtomicBool,
    /// Whether any traced values have been written yet.
    initialized: AtomicBool,
}

impl Default for AcousticDspParams {
    /// Starts clear and dry, with processing enabled.
    fn default() -> Self {
        Self {
            gain_lf: AtomicU32::new(1.0_f32.to_bits()),
            gain_hf: AtomicU32::new(1.0_f32.to_bits()),
            wet_gain: AtomicU32::new(0.0_f32.to_bits()),
            decay_time_s: AtomicU32::new(0.5_f32.to_bits()),
            pre_delay_s: AtomicU32::new(0.0_f32.to_bits()),
            decay_low_s: AtomicU32::new(0.5_f32.to_bits()),
            decay_high_s: AtomicU32::new(0.3_f32.to_bits()),
            early_gain: AtomicU32::new(0.0_f32.to_bits()),
            enabled: AtomicBool::new(true),
            initialized: AtomicBool::new(false),
        }
    }
}

impl AcousticDspParams {
    /// Writes the latest muffle filter.
    pub fn set_filter(&self, filter: MuffleFilter) {
        self.gain_lf
            .store(filter.gain_lf().to_bits(), Ordering::Relaxed);
        self.gain_hf
            .store(filter.gain_hf().to_bits(), Ordering::Relaxed);
        self.initialized.store(true, Ordering::Release);
    }

    /// Writes the latest reverb estimate, scaling its wet gain by `send`.
    ///
    /// The send is clamped to `[0, 2]`; non-finite values use unity gain.
    pub fn set_reverb(&self, reverb: ReverbEstimate, send: f32) {
        let send = if send.is_finite() {
            send.clamp(0.0, 2.0)
        } else {
            1.0
        };
        let wet = (reverb.wet_gain() * send).clamp(0.0, 1.0);
        self.wet_gain.store(wet.to_bits(), Ordering::Relaxed);
        self.early_gain.store(
            (reverb.early_gain() * send).clamp(0.0, 1.0).to_bits(),
            Ordering::Relaxed,
        );
        self.decay_low_s
            .store(reverb.decay_low_s().to_bits(), Ordering::Relaxed);
        self.decay_high_s
            .store(reverb.decay_high_s().to_bits(), Ordering::Relaxed);
        self.decay_time_s
            .store(reverb.decay_time_s().to_bits(), Ordering::Relaxed);
        self.pre_delay_s.store(
            reverb
                .reflections_delay_s()
                .clamp(0.0, MAXIMUM_PRE_DELAY_S)
                .to_bits(),
            Ordering::Relaxed,
        );
    }

    /// Enables or bypasses processing.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Returns whether processing is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Returns the current muffle filter.
    #[must_use]
    pub fn filter(&self) -> MuffleFilter {
        MuffleFilter::try_new(
            load(&self.gain_lf).clamp(0.0, 1.0),
            load(&self.gain_hf).clamp(0.0, 1.0),
        )
        .unwrap_or(MuffleFilter::CLEAR)
    }

    /// Returns the current reverb send gain.
    #[must_use]
    pub fn wet_gain(&self) -> f32 {
        load(&self.wet_gain)
    }

    /// Returns a snapshot of all values for the audio thread.
    fn snapshot(&self) -> Targets {
        let enabled = self.enabled.load(Ordering::Relaxed);
        if !enabled {
            return Targets::BYPASS;
        }
        Targets {
            gain_lf: load(&self.gain_lf),
            gain_hf: load(&self.gain_hf),
            wet_gain: load(&self.wet_gain),
            decay_time_s: load(&self.decay_time_s).max(0.05),
            decay_low_s: load(&self.decay_low_s).max(0.05),
            decay_high_s: load(&self.decay_high_s).max(0.05),
            early_gain: load(&self.early_gain),
            pre_delay_s: load(&self.pre_delay_s).clamp(0.0, MAXIMUM_PRE_DELAY_S),
        }
    }
}

/// Loads one `f32` stored as bits.
fn load(value: &AtomicU32) -> f32 {
    f32::from_bits(value.load(Ordering::Relaxed))
}

/// Parameter values the processor smooths toward.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Targets {
    /// Low-frequency gain.
    gain_lf: f32,
    /// High-frequency gain.
    gain_hf: f32,
    /// Reverb send gain.
    wet_gain: f32,
    /// 60 dB decay time in seconds.
    decay_time_s: f32,
    /// Low-frequency decay time.
    decay_low_s: f32,
    /// High-frequency decay time.
    decay_high_s: f32,
    /// Early-return amplitude.
    early_gain: f32,
    /// Pre-delay in seconds.
    pre_delay_s: f32,
}

impl Targets {
    /// Values that leave audio unchanged.
    const BYPASS: Self = Self {
        gain_lf: 1.0,
        gain_hf: 1.0,
        wet_gain: 0.0,
        decay_time_s: 0.5,
        decay_low_s: 0.5,
        decay_high_s: 0.3,
        early_gain: 0.0,
        pre_delay_s: 0.0,
    };
}

/// A feedback comb filter with a one-pole damping filter in its loop.
#[derive(Clone, Debug)]
struct Comb {
    /// Circular delay buffer.
    buffer: Vec<f32>,
    /// Next read and write position.
    position: usize,
    /// Damping filter state.
    filter_state: f32,
}

impl Comb {
    /// Processes one sample with the given feedback gain.
    fn process(&mut self, input: f32, feedback: [f32; 2], crossover: f32) -> f32 {
        let Some(slot) = self.buffer.get_mut(self.position) else {
            return 0.0;
        };
        let output = *slot;
        self.filter_state += crossover * (output - self.filter_state);
        let [low_gain, high_gain] = feedback;
        let high = output - self.filter_state;
        *slot = self.filter_state * low_gain + high * high_gain + input;
        self.position += 1;
        if self.position >= self.buffer.len() {
            self.position = 0;
        }
        output
    }

    /// Returns the loop delay in seconds.
    fn delay_s(&self, sample_rate: f32) -> f32 {
        count_to_f32(self.buffer.len()) / sample_rate
    }
}

/// A Schroeder all-pass diffuser.
#[derive(Clone, Debug)]
struct AllPass {
    /// Circular delay buffer.
    buffer: Vec<f32>,
    /// Next read and write position.
    position: usize,
}

impl AllPass {
    /// Processes one sample.
    fn process(&mut self, input: f32) -> f32 {
        let Some(slot) = self.buffer.get_mut(self.position) else {
            return input;
        };
        let delayed = *slot;
        let output = delayed - input;
        *slot = delayed * ALLPASS_FEEDBACK + input;
        self.position += 1;
        if self.position >= self.buffer.len() {
            self.position = 0;
        }
        output
    }
}

/// Processes interleaved samples for one sound with smoothed muffling and reverb.
#[derive(Debug)]
pub struct AcousticDspProcessor {
    /// Shared parameters written by the game thread.
    params: std::sync::Arc<AcousticDspParams>,
    /// Number of interleaved channels.
    channels: usize,
    /// Output sample rate in hertz.
    sample_rate: f32,
    /// Channel index of the next input sample.
    channel: usize,
    /// Frames since the parameters were last read.
    frames_since_refresh: u32,
    /// Most recently read target values.
    targets: Targets,
    /// Smoothed values currently applied.
    current: Targets,
    /// Per-sample smoothing coefficient.
    smoothing: f32,
    /// One-pole low-pass coefficient for the crossover.
    crossover: f32,
    /// First low-pass stage state per channel.
    low_first: Vec<f32>,
    /// Second low-pass stage state per channel.
    low_second: Vec<f32>,
    /// Mono reverb input summed over the current frame.
    reverb_input: f32,
    /// Reverb output for the current frame.
    reverb_output: f32,
    /// Pre-delay circular buffer.
    pre_delay: Vec<f32>,
    /// Pre-delay write position.
    pre_delay_position: usize,
    /// Parallel comb filters.
    combs: Vec<Comb>,
    /// Series all-pass diffusers.
    allpasses: Vec<AllPass>,
    /// Comb feedback gains derived from the smoothed decay time.
    comb_feedback: [[f32; 2]; 4],
    /// Decay time used for the current comb feedback gains.
    feedback_decay_s: [f32; 2],
}

impl AcousticDspProcessor {
    /// Allocates delay lines for one sound's channel count and sample rate.
    #[must_use]
    pub fn new(
        params: std::sync::Arc<AcousticDspParams>,
        channels: u16,
        sample_rate_hz: u32,
    ) -> Self {
        let channels = usize::from(channels.max(1));
        let sample_rate = rate_to_f32(sample_rate_hz.max(1));
        let scale = sample_rate / 44_100.0;
        let scaled =
            |length: usize| samples_to_count((count_to_f32(length) * scale).round()).max(1);
        // Starting at the current targets avoids an audible sweep when a sound starts.
        let initial = params.snapshot();
        let mut processor = Self {
            params,
            channels,
            sample_rate,
            channel: 0,
            frames_since_refresh: 0,
            targets: initial,
            current: initial,
            smoothing: 1.0 - (-1.0 / (SMOOTHING_S * sample_rate)).exp(),
            crossover: 1.0 - (-core::f32::consts::TAU * CROSSOVER_HZ / sample_rate).exp(),
            low_first: vec![0.0; channels],
            low_second: vec![0.0; channels],
            reverb_input: 0.0,
            reverb_output: 0.0,
            pre_delay: vec![0.0; samples_to_count((MAXIMUM_PRE_DELAY_S * sample_rate).ceil()) + 1],
            pre_delay_position: 0,
            combs: COMB_LENGTHS_44K
                .iter()
                .map(|length| Comb {
                    buffer: vec![0.0; scaled(*length)],
                    position: 0,
                    filter_state: 0.0,
                })
                .collect(),
            allpasses: ALLPASS_LENGTHS_44K
                .iter()
                .map(|length| AllPass {
                    buffer: vec![0.0; scaled(*length)],
                    position: 0,
                })
                .collect(),
            comb_feedback: [[0.0; 2]; 4],
            feedback_decay_s: [0.0; 2],
        };
        processor.update_feedback();
        processor
    }

    /// Processes one interleaved sample and returns the output sample.
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let (direct, reverberant) = self.process_sample_split(input);
        direct + reverberant
    }

    /// Processes a sample, returning the direct and reverberant contributions separately.
    ///
    /// Headphone renderers can localize the direct signal while leaving the room return diffuse.
    pub fn process_sample_split(&mut self, input: f32) -> (f32, f32) {
        if self.channel == 0 {
            self.begin_frame();
        }
        let channel = self.channel;
        self.channel += 1;
        if self.channel >= self.channels {
            self.channel = 0;
        }

        let Some(first) = self.low_first.get_mut(channel) else {
            return (input, 0.0);
        };
        *first += self.crossover * (input - *first);
        let first_output = *first;
        let Some(second) = self.low_second.get_mut(channel) else {
            return (input, 0.0);
        };
        *second += self.crossover * (first_output - *second);
        let low = *second;
        let high = input - low;
        let dry = low * self.current.gain_lf + high * self.current.gain_hf;

        self.reverb_input += dry;
        (dry, self.reverb_output)
    }

    /// Smooths parameters and advances the reverb by one frame using the previous frame's input.
    fn begin_frame(&mut self) {
        if self.frames_since_refresh == 0 {
            self.targets = self.params.snapshot();
        }
        self.frames_since_refresh += 1;
        if self.frames_since_refresh >= PARAMETER_REFRESH_FRAMES {
            self.frames_since_refresh = 0;
        }

        let rate = self.smoothing;
        let current = &mut self.current;
        let targets = self.targets;
        current.gain_lf += rate * (targets.gain_lf - current.gain_lf);
        current.gain_hf += rate * (targets.gain_hf - current.gain_hf);
        current.wet_gain += rate * (targets.wet_gain - current.wet_gain);
        current.decay_time_s += rate * (targets.decay_time_s - current.decay_time_s);
        current.pre_delay_s += rate * (targets.pre_delay_s - current.pre_delay_s);
        current.decay_low_s += rate * (targets.decay_low_s - current.decay_low_s);
        current.decay_high_s += rate * (targets.decay_high_s - current.decay_high_s);
        current.early_gain += rate * (targets.early_gain - current.early_gain);
        let [low, high] = self.feedback_decay_s;
        if (current.decay_low_s - low).abs() > 0.01 || (current.decay_high_s - high).abs() > 0.01 {
            self.update_feedback();
        }

        let mono = self.reverb_input / count_to_f32(self.channels);
        self.reverb_input = 0.0;
        self.reverb_output = self.reverb_frame(mono);
    }

    /// Runs one mono sample through the pre-delay, combs, and all-passes.
    fn reverb_frame(&mut self, input: f32) -> f32 {
        let length = self.pre_delay.len();
        if let Some(slot) = self.pre_delay.get_mut(self.pre_delay_position) {
            *slot = input;
        }
        let delay_samples = samples_to_count(self.current.pre_delay_s * self.sample_rate)
            .min(length.saturating_sub(1));
        let read = (self.pre_delay_position + length - delay_samples) % length.max(1);
        let delayed = self.pre_delay.get(read).copied().unwrap_or(0.0);
        self.pre_delay_position += 1;
        if self.pre_delay_position >= length {
            self.pre_delay_position = 0;
        }

        let mut sum = 0.0;
        for (comb, feedback) in self.combs.iter_mut().zip(self.comb_feedback) {
            sum += comb.process(delayed, feedback, self.crossover);
        }
        let mut output = sum * 0.25;
        for allpass in &mut self.allpasses {
            output = allpass.process(output);
        }
        // The early return remains audible near outdoor walls even when the diffuse tail is dry.
        output * REVERB_OUTPUT_SCALE * self.current.wet_gain + delayed * self.current.early_gain
    }

    /// Recomputes comb feedback so each comb decays by 60 dB over the smoothed decay time.
    fn update_feedback(&mut self) {
        let decays = [
            self.current.decay_low_s.max(0.05),
            self.current.decay_high_s.max(0.05),
        ];
        for (feedback, comb) in self.comb_feedback.iter_mut().zip(&self.combs) {
            *feedback = decays.map(|decay| {
                10.0_f32
                    .powf(-3.0 * comb.delay_s(self.sample_rate) / decay)
                    .min(0.98)
            });
        }
        self.feedback_decay_s = decays;
    }
}

#[cfg(test)]
#[path = "../tests/unit/dsp.rs"]
mod tests;
