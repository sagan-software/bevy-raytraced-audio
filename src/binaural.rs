//! Measured, interpolated KEMAR headphone rendering for listener-local 3D directions.
//!
//! Coordinates follow Bevy: +X right, +Y up, -Z forward. The MIT data supplies
//! interaural timing, head shadow and pinna spectral cues. See `src/hrtf/README.md`
//! for attribution and the measured elevation range (-40 to +90 degrees).

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Diffuse-field equalized MIT KEMAR responses, sorted by elevation then azimuth.
const DATA: &[u8] = include_bytes!("hrtf/kemar.bin");
/// Samples per ear in a native-rate response.
const TAPS: usize = 128;
/// Bytes per record: elevation, azimuth, then interleaved signed 16-bit PCM.
const RECORD_BYTES: usize = 4 + TAPS * 4;
/// Native measurement sample rate.
const MEASUREMENT_RATE: f32 = 44_100.0;
/// Frames between position refreshes; convolution coefficients are smoothed every sample.
const REFRESH_FRAMES: u32 = 256;

/// Lock-free listener-local position and headphone-mode controls for one point source.
#[derive(Debug)]
pub struct BinauralParams {
    /// Rightward distance, stored as float bits.
    x: AtomicU32,
    /// Upward distance, stored as float bits.
    y: AtomicU32,
    /// Backward distance, stored as float bits.
    z: AtomicU32,
    /// Scaled meters per world unit, matching the scene's desired distance rolloff.
    distance_scale: f32,
    /// Whether to use measured HRTFs instead of equal-power stereo panning.
    enabled: AtomicBool,
    /// Whether a unique finite listener and emitter currently exist.
    valid: AtomicBool,
}

impl BinauralParams {
    /// Creates headphone controls with inverse-distance attenuation beyond one scaled meter.
    ///
    /// `distance_scale` must be positive and finite; invalid values use `1`.
    /// A source remains silent until [`Self::set_position`] supplies a valid listener-local pose.
    #[must_use]
    pub fn new(distance_scale: f32) -> Self {
        Self {
            x: AtomicU32::new(0.0_f32.to_bits()),
            y: AtomicU32::new(0.0_f32.to_bits()),
            z: AtomicU32::new((-1.0_f32).to_bits()),
            distance_scale: if distance_scale.is_finite() && distance_scale > 0.0 {
                distance_scale
            } else {
                1.0
            },
            enabled: AtomicBool::new(true),
            valid: AtomicBool::new(false),
        }
    }

    /// Sets the emitter relative to the listener's head (+X right, +Y up, -Z forward).
    ///
    /// Non-finite positions mute the source until a valid position arrives.
    pub fn set_position(&self, position: [f32; 3]) {
        let [x, y, z] = position;
        let valid = position.into_iter().all(f32::is_finite);
        if valid {
            self.x.store(x.to_bits(), Ordering::Relaxed);
            self.y.store(y.to_bits(), Ordering::Relaxed);
            self.z.store(z.to_bits(), Ordering::Relaxed);
        }
        self.valid.store(valid, Ordering::Release);
    }

    /// Mutes this point source while no unique listener is available.
    pub fn clear_listener(&self) {
        self.valid.store(false, Ordering::Release);
    }

    /// Selects measured headphone rendering or equal-power stereo for comparison.
    ///
    /// Both modes retain distance attenuation and acoustic filtering. This is independent
    /// of [`crate::AcousticDspParams::set_enabled`], which bypasses room processing.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Returns whether measured headphone filtering is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Reads a bounded snapshot; slight inter-field skew is removed by coefficient smoothing.
    fn snapshot(&self) -> Position {
        let valid = self.valid.load(Ordering::Acquire);
        let x = f32::from_bits(self.x.load(Ordering::Relaxed));
        let y = f32::from_bits(self.y.load(Ordering::Relaxed));
        let z = f32::from_bits(self.z.load(Ordering::Relaxed));
        let horizontal = x.hypot(z);
        let distance = horizontal.hypot(y);
        Position {
            azimuth: if horizontal > 1.0e-6 {
                x.atan2(-z).to_degrees()
            } else {
                0.0
            },
            elevation: y.atan2(horizontal).to_degrees().clamp(-40.0, 90.0),
            pan: if distance > 1.0e-6 { x / distance } else { 0.0 },
            gain: if valid {
                (distance * self.distance_scale).max(1.0).recip()
            } else {
                0.0
            },
            enabled: self.is_enabled(),
        }
    }
}

/// One finite snapshot of the listener-relative source.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    /// Signed azimuth, positive toward the right ear.
    azimuth: f32,
    /// Elevation clamped to the measured range.
    elevation: f32,
    /// Equal-power fallback pan.
    pan: f32,
    /// Common amplitude attenuation.
    gain: f32,
    /// Whether measured filtering is active.
    enabled: bool,
}

/// Decodes one little-endian integer from the embedded, validated table.
fn integer(offset: usize) -> i16 {
    let lo = DATA.get(offset).copied().unwrap_or(0);
    let hi = DATA.get(offset + 1).copied().unwrap_or(0);
    i16::from_le_bytes([lo, hi])
}

/// One measurement and its interpolation weight.
#[derive(Clone, Copy, Debug, Default)]
struct WeightedRecord {
    /// Record index into the embedded table.
    index: usize,
    /// Contribution to the output response.
    weight: f32,
    /// Mirrors a virtual measurement across the rear median plane.
    swap_ears: bool,
}

/// Finds the azimuth neighbors on an elevation ring, preserving the irregular measurement grid.
fn ring(elevation: i16, azimuth: f32, weight: f32) -> [WeightedRecord; 2] {
    let mut lower = None;
    let mut upper = None;
    for index in 0..DATA.len() / RECORD_BYTES {
        let offset = index * RECORD_BYTES;
        if integer(offset) != elevation {
            continue;
        }
        let angle = f32::from(integer(offset + 2));
        if angle <= azimuth {
            lower = Some((index, angle));
        }
        if angle >= azimuth {
            upper = Some((index, angle));
            break;
        }
    }
    let (lo, a) = lower.or(upper).unwrap_or((0, 0.0));
    // The +50 degree ring ends at 176 rather than 180 degrees. Interpolate to
    // its mirrored 184-degree response so crossing the rear seam stays continuous.
    let swap_ears = upper.is_none();
    let (hi, b) = upper.unwrap_or((lo, 360.0 - a));
    let blend = if b > a { (azimuth - a) / (b - a) } else { 0.0 };
    [
        WeightedRecord {
            index: lo,
            weight: weight * (1.0 - blend),
            swap_ears: false,
        },
        WeightedRecord {
            index: hi,
            weight: weight * blend,
            swap_ears,
        },
    ]
}

/// Bilinearly interpolates measured responses, swapping ears across the median plane.
#[expect(
    clippy::suboptimal_flops,
    reason = "bounded audio rounding avoids per-tap software FMA dispatch"
)]
fn response(position: Position) -> [[f32; 2]; TAPS] {
    let mut lower = -40;
    let mut upper = 90;
    for elevation in (-40..=90).step_by(10) {
        if f32::from(elevation) <= position.elevation {
            lower = elevation;
        }
        if f32::from(elevation) >= position.elevation {
            upper = elevation;
            break;
        }
    }
    let blend = if upper > lower {
        (position.elevation - f32::from(lower)) / f32::from(upper - lower)
    } else {
        0.0
    };
    let low = ring(lower, position.azimuth.abs(), 1.0 - blend);
    let high = ring(upper, position.azimuth.abs(), blend);
    let mut taps = [[0.0; 2]; TAPS];
    for record in low.into_iter().chain(high) {
        for (tap, output) in taps.iter_mut().enumerate() {
            for (ear, sample) in output.iter_mut().enumerate() {
                let measured_ear = if (position.azimuth < 0.0) == record.swap_ears {
                    ear
                } else {
                    1 - ear
                };
                let offset = record.index * RECORD_BYTES + 4 + tap * 4 + measured_ear * 2;
                *sample += f32::from(integer(offset)) * (record.weight / 32_768.0);
            }
        }
    }
    taps
}

/// One cached windowed-sinc resampling tap, built outside the audio callback.
#[derive(Clone, Debug)]
struct ResampleTap {
    /// Native-response sample indices and weights, including anti-aliasing for low rates.
    weights: Vec<(usize, f32)>,
}

/// Renders mono point sources to stereo with measured elevation and azimuth cues.
///
/// Allocate once per voice. [`Self::process_frame`] does not allocate or lock. Responses
/// are resampled to the voice's sample rate; moving heads and sources smoothly change
/// the FIR coefficients. Stereo recordings should be downmixed before calling this.
#[derive(Debug)]
pub struct BinauralProcessor {
    /// Position controls shared with the game thread.
    params: Arc<BinauralParams>,
    /// Latest target position.
    position: Position,
    /// Smoothed FIR coefficients, paired left and right.
    current: Vec<[f32; 2]>,
    /// Desired FIR coefficients at the stream rate.
    target: Vec<[f32; 2]>,
    /// Cached native-to-stream-rate interpolation weights.
    resampler: Vec<ResampleTap>,
    /// Doubled circular history keeps the convolution's read window contiguous.
    history: Vec<f32>,
    /// First sample of the current history window.
    cursor: usize,
    /// Frames since the last position refresh.
    frames: u32,
    /// Per-frame smoothing coefficient.
    smoothing: f32,
    /// Whether a changed direction requires per-sample coefficient interpolation.
    smoothing_active: bool,
    /// Whether every input so far has been finite; preserves non-finite FIR propagation.
    finite_history: bool,
    /// Smoothed common gain, also applied to the diffuse reverb.
    gain: f32,
}

impl BinauralProcessor {
    /// Allocates convolution state for a mono point source at the supplied sample rate.
    ///
    /// Sample rates are bounded to 8–192 kHz; normal decoded game audio falls in this range.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded audio sample rates and sub-millisecond FIR indices fit in these types"
    )]
    pub fn new(params: Arc<BinauralParams>, sample_rate_hz: u32) -> Self {
        let rate = sample_rate_hz.clamp(8_000, 192_000) as f32;
        let ratio = MEASUREMENT_RATE / rate;
        let length = (TAPS as f32 / ratio).ceil() as usize;
        let cutoff = ratio.recip().min(1.0);
        let resampler = (0..length)
            .map(|index| {
                let at = index as f32 * ratio;
                let weights = (0..TAPS)
                    .filter_map(|tap| {
                        let delta = (at - tap as f32) * cutoff;
                        if delta.abs() >= 8.0 {
                            return None;
                        }
                        let sinc = if delta.abs() < 1.0e-6 {
                            1.0
                        } else {
                            let phase = core::f32::consts::PI * delta;
                            phase.sin() / phase
                        };
                        let window =
                            f32::midpoint(1.0, (core::f32::consts::PI * delta / 8.0).cos());
                        Some((tap, sinc * window * cutoff * ratio))
                    })
                    .collect();
                ResampleTap { weights }
            })
            .collect();
        let position = params.snapshot();
        let mut result = Self {
            params,
            position,
            current: vec![[0.0; 2]; length],
            target: vec![[0.0; 2]; length],
            resampler,
            history: vec![0.0; length * 2],
            cursor: 0,
            frames: 0,
            smoothing: 1.0 - (-1.0 / (0.015 * rate)).exp(),
            gain: position.gain,
            smoothing_active: false,
            finite_history: true,
        };
        result.update_response();
        result.current.clone_from(&result.target);
        result
    }

    /// Renders one filtered mono sample and an unlocalized diffuse reverb sample to stereo.
    ///
    /// # Panics
    /// The internal history window must match the constructor-allocated filter length.
    #[expect(
        clippy::suboptimal_flops,
        reason = "explicit multiply/add avoids software fmaf calls in the per-tap WebAssembly loop"
    )]
    pub fn process_frame(&mut self, direct: f32, diffuse: f32) -> [f32; 2] {
        if self.frames == 0 {
            let position = self.params.snapshot();
            if position != self.position {
                let changed = position.azimuth.to_bits() != self.position.azimuth.to_bits()
                    || position.elevation.to_bits() != self.position.elevation.to_bits()
                    || position.pan.to_bits() != self.position.pan.to_bits()
                    || position.enabled != self.position.enabled;
                self.position = position;
                if changed {
                    self.smoothing_active = true;
                    self.update_response();
                }
            }
        }
        self.frames = (self.frames + 1) % REFRESH_FRAMES;
        self.gain += self.smoothing * (self.position.gain - self.gain);
        let length = self.current.len();
        self.cursor = if self.cursor == 0 {
            length - 1
        } else {
            self.cursor - 1
        };
        if let Some(slot) = self.history.get_mut(self.cursor) {
            *slot = direct;
        }
        if let Some(slot) = self.history.get_mut(self.cursor + length) {
            *slot = direct;
        }
        let history = self
            .history
            .get(self.cursor..self.cursor + length)
            .expect("doubled history window");
        self.finite_history &= direct.is_finite();
        let filtered = if !self.smoothing_active && !self.position.enabled && self.finite_history {
            // A never-transitioned stereo fallback has exactly one nonzero FIR tap.
            self.current
                .first()
                .copied()
                .unwrap_or([0.; 2])
                .map(|coefficient| direct * coefficient)
        } else if self.smoothing_active {
            convolve::<true>(&mut self.current, &self.target, history, self.smoothing)
        } else {
            convolve::<false>(&mut self.current, &self.target, history, self.smoothing)
        };
        let stereo = filtered.map(|sample| sample + diffuse * core::f32::consts::FRAC_1_SQRT_2);
        stereo.map(|sample| sample * self.gain)
    }

    /// Rebuilds target filters in existing storage; the callback never allocates.
    #[expect(
        clippy::suboptimal_flops,
        reason = "bounded audio rounding avoids per-tap software FMA dispatch"
    )]
    fn update_response(&mut self) {
        if !self.position.enabled {
            self.target.fill([0.0; 2]);
            if let Some(first) = self.target.first_mut() {
                *first = [
                    ((1.0 - self.position.pan) * 0.5).sqrt(),
                    f32::midpoint(1.0, self.position.pan).sqrt(),
                ];
            }
            return;
        }
        let native = response(self.position);
        for (output, resample) in self.target.iter_mut().zip(&self.resampler) {
            *output = [0.0; 2];
            for &(index, weight) in &resample.weights {
                if let Some(tap) = native.get(index) {
                    for (sample, value) in output.iter_mut().zip(tap) {
                        *sample += value * weight * core::f32::consts::FRAC_1_SQRT_2;
                    }
                }
            }
        }
    }
}

/// Four independent accumulation lanes expose SIMD without changing FIR length or smoothing.
///
/// Only the summation order changes. A scalar reference test bounds the rounding difference.
#[expect(
    clippy::suboptimal_flops,
    reason = "portable multiply/add avoids software FMA in the audio callback"
)]
fn convolve<const SMOOTH: bool>(
    current: &mut [[f32; 2]],
    target: &[[f32; 2]],
    input: &[f32],
    smoothing: f32,
) -> [f32; 2] {
    let (current, tail) = current.as_chunks_mut::<4>();
    let (target, target_tail) = target.as_chunks::<4>();
    let (input, input_tail) = input.as_chunks::<4>();
    let mut sums = [[0.0; 2]; 4];
    for ((coefficients, goals), samples) in current.iter_mut().zip(target).zip(input) {
        for (((coefficient, goal), sample), sum) in coefficients
            .iter_mut()
            .zip(goals)
            .zip(samples)
            .zip(&mut sums)
        {
            for ((value, desired), output) in coefficient.iter_mut().zip(goal).zip(sum) {
                if SMOOTH {
                    *value += smoothing * (*desired - *value);
                }
                *output += *value * *sample;
            }
        }
    }
    let mut output = [0.0; 2];
    for sum in sums {
        for (output, value) in output.iter_mut().zip(sum) {
            *output += value;
        }
    }
    for ((coefficient, goal), sample) in tail.iter_mut().zip(target_tail).zip(input_tail) {
        for ((value, desired), output) in coefficient.iter_mut().zip(goal).zip(&mut output) {
            if SMOOTH {
                *value += smoothing * (*desired - *value);
            }
            *output += *value * *sample;
        }
    }
    output
}

#[cfg(test)]
#[path = "../tests/unit/binaural.rs"]
mod tests;
