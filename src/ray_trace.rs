//! Listener-centred stochastic ray tracing settings and dimension-independent results.
//!
//! A listener trace fires rays outward from the listener and follows their specular, optionally
//! scattered, bounces. Every bounce point spawns three kinds of secondary checks:
//!
//! - **Occlusion** rays test line of sight from the bounce point to each source. The fraction of
//!   primary rays that discover a source sets how clear, or how muffled, that source sounds.
//! - **Echo** rays test line of sight from the bounce point back to the listener. Their count and
//!   path lengths estimate reverb loudness, early-reflection delay, and decay time.
//! - **Permeation** rays travel straight through walls from the listener and early bounce points to
//!   each source. Material transmission along the way sets how much sound leaks through walls.
//!
//! Rays that leave the scene without hitting a surface count as escaped. The escaped fraction
//! estimates how outdoor the listener's position is, and the last echo point before an escape
//! gives the direction outdoor ambience such as rain should come from.

use crate::{BandGain, GeometryError};

/// Speed of sound in dry air at 20 °C, in meters per second.
pub const SPEED_OF_SOUND_M_PER_S: f64 = 343.0;

/// Natural logarithm of the 60 dB energy ratio used by reverberation-time formulas.
const SIXTY_DECIBEL_LOG: f64 = 13.815_510_557_964_274;

/// Shortest reported decay time in seconds.
const MINIMUM_DECAY_S: f64 = 0.05;

/// Longest reported decay time in seconds.
const MAXIMUM_DECAY_S: f64 = 8.0;

/// Longest reported early-reflection delay in seconds.
const MAXIMUM_REFLECTIONS_DELAY_S: f64 = 0.3;

/// Exponential rate mapping the discovered-ray fraction to muffle strength.
const MUFFLE_CURVE_RATE: f32 = 6.0;

/// Narrows a solver statistic to the `f32` precision used by public results.
#[expect(
    clippy::cast_possible_truncation,
    reason = "statistics are bounded ratios, distances, and times well inside f32 range"
)]
pub(crate) const fn narrow(value: f64) -> f32 {
    value as f32
}

/// Ray counts, bounce limits, and recording options for one listener trace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayTraceSettings {
    /// Number of primary rays fired from the listener.
    ray_count: u32,
    /// Maximum surface hits followed per primary ray.
    max_bounces: u32,
    /// Distance after which a ray that hits nothing counts as escaped, in meters.
    escape_distance_m: f64,
    /// Fraction of each reflection direction replaced by a random diffuse direction.
    scattering: f32,
    /// Number of primary rays whose first bounce point also casts permeation rays.
    permeation_ray_count: u32,
    /// Seed for the deterministic direction offset and diffuse scattering.
    seed: u64,
    /// Whether the trace records every ray segment for visualization.
    record_rays: bool,
}

impl Default for RayTraceSettings {
    /// Uses 256 rays, 8 bounces, a 100 m escape distance, and light scattering.
    fn default() -> Self {
        Self {
            ray_count: 256,
            max_bounces: 8,
            escape_distance_m: 100.0,
            scattering: 0.1,
            permeation_ray_count: 16,
            seed: 0x5EED_A0D1_0000_0001,
            record_rays: false,
        }
    }
}

impl RayTraceSettings {
    /// Sets the primary ray count.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::CoefficientOutOfRange`] when the count is zero.
    pub const fn try_with_ray_count(mut self, ray_count: u32) -> Result<Self, GeometryError> {
        if ray_count == 0 {
            return Err(GeometryError::CoefficientOutOfRange);
        }
        self.ray_count = ray_count;
        Ok(self)
    }

    /// Sets the maximum number of surface hits followed per primary ray.
    #[must_use]
    pub const fn with_max_bounces(mut self, max_bounces: u32) -> Self {
        self.max_bounces = max_bounces;
        self
    }

    /// Sets the distance after which a ray that hits nothing counts as escaped.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite distance or
    /// [`GeometryError::CoefficientOutOfRange`] for a distance that is not positive.
    pub fn try_with_escape_distance(mut self, distance_m: f64) -> Result<Self, GeometryError> {
        if !distance_m.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }
        if distance_m <= 0.0 {
            return Err(GeometryError::CoefficientOutOfRange);
        }
        self.escape_distance_m = distance_m;
        Ok(self)
    }

    /// Sets the diffuse fraction of each reflection.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite value or
    /// [`GeometryError::CoefficientOutOfRange`] outside `[0, 1]`.
    pub fn try_with_scattering(mut self, scattering: f32) -> Result<Self, GeometryError> {
        if !scattering.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }
        if !(0.0..=1.0).contains(&scattering) {
            return Err(GeometryError::CoefficientOutOfRange);
        }
        self.scattering = scattering;
        Ok(self)
    }

    /// Sets how many primary rays cast permeation rays from their first bounce point.
    #[must_use]
    pub const fn with_permeation_ray_count(mut self, count: u32) -> Self {
        self.permeation_ray_count = count;
        self
    }

    /// Sets the seed used for direction jitter and diffuse scattering.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Enables or disables recording ray segments for visualization.
    #[must_use]
    pub const fn with_recorded_rays(mut self, record_rays: bool) -> Self {
        self.record_rays = record_rays;
        self
    }

    /// Returns the primary ray count.
    #[must_use]
    pub const fn ray_count(self) -> u32 {
        self.ray_count
    }

    /// Returns the bounce limit per primary ray.
    #[must_use]
    pub const fn max_bounces(self) -> u32 {
        self.max_bounces
    }

    /// Returns the escape distance in meters.
    #[must_use]
    pub const fn escape_distance_m(self) -> f64 {
        self.escape_distance_m
    }

    /// Returns the diffuse fraction of each reflection.
    #[must_use]
    pub const fn scattering(self) -> f32 {
        self.scattering
    }

    /// Returns how many primary rays cast permeation rays from their first bounce.
    #[must_use]
    pub const fn permeation_ray_count(self) -> u32 {
        self.permeation_ray_count
    }

    /// Returns the deterministic seed.
    #[must_use]
    pub const fn seed(self) -> u64 {
        self.seed
    }

    /// Returns whether ray segments are recorded for visualization.
    #[must_use]
    pub const fn records_rays(self) -> bool {
        self.record_rays
    }
}

/// The role a recorded ray segment played in a listener trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RayKind {
    /// A primary ray leg from the listener or a bounce point to the next surface hit.
    Primary,
    /// A primary ray leg that left the scene without hitting a surface.
    Escaped,
    /// An unobstructed line of sight from a bounce point to a source.
    Occlusion,
    /// An unobstructed line of sight from a bounce point back to the listener.
    Echo,
    /// A straight path through walls from the listener or a bounce point to a source.
    Permeation,
    /// The direction outdoor ambience arrives from for one escaped ray.
    Ambient,
}

/// Two-band amplitude gains that describe how clear or muffled a source sounds.
///
/// `gain_lf` scales content below the crossover and `gain_hf` scales content above it, matching
/// the low-pass gain and high-frequency gain pair used by common game audio filters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MuffleFilter {
    /// Linear gain applied to low frequencies.
    gain_lf: f32,
    /// Linear gain applied to high frequencies.
    gain_hf: f32,
}

impl MuffleFilter {
    /// An unfiltered, fully clear path.
    pub const CLEAR: Self = Self {
        gain_lf: 1.0,
        gain_hf: 1.0,
    };

    /// A fully silent path.
    pub const SILENT: Self = Self {
        gain_lf: 0.0,
        gain_hf: 0.0,
    };

    /// Builds a filter from bounded low- and high-frequency gains.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite gain or
    /// [`GeometryError::CoefficientOutOfRange`] outside `[0, 1]`.
    pub fn try_new(low_gain: f32, high_gain: f32) -> Result<Self, GeometryError> {
        BandGain::try_new(low_gain, low_gain, high_gain)?;
        Ok(Self {
            gain_lf: low_gain,
            gain_hf: high_gain,
        })
    }

    /// Returns the low-frequency linear gain.
    #[must_use]
    pub const fn gain_lf(self) -> f32 {
        self.gain_lf
    }

    /// Returns the high-frequency linear gain.
    #[must_use]
    pub const fn gain_hf(self) -> f32 {
        self.gain_hf
    }

    /// Combines occlusion clarity with permeation transmission into one filter.
    ///
    /// Clarity `c` contributes `sqrt(c)` to low frequencies and `c * sqrt(c)` to high frequencies,
    /// so partly hidden sources lose treble before bass. Permeation adds its low and high band
    /// amplitudes in energy, and both bands are clamped to `[0, 1]`.
    #[must_use]
    pub fn from_clarity_and_permeation(clarity: f32, permeation: BandGain) -> Self {
        let clarity = clarity.clamp(0.0, 1.0);
        let bass_clarity = clarity.sqrt();
        let treble_clarity = clarity * bass_clarity;
        Self {
            gain_lf: bass_clarity.hypot(permeation.low()).min(1.0),
            gain_hf: treble_clarity.hypot(permeation.high()).min(1.0),
        }
    }
}

/// How one source sounds from the listener after a ray trace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceRayResponse {
    /// Whether the listener has an unobstructed straight line to the source.
    direct_visible: bool,
    /// Energy-weighted fraction of primary rays that discovered the source, in `[0, 1]`.
    discovered_fraction: f32,
    /// Clarity derived from line of sight and discovered rays, in `[0, 1]`.
    clarity: f32,
    /// Mean amplitude transmission of permeation rays by band.
    permeation: BandGain,
    /// Combined filter to apply to the source.
    filter: MuffleFilter,
}

impl SourceRayResponse {
    /// A response for a source that has not been traced.
    pub const UNTRACED: Self = Self {
        direct_visible: true,
        discovered_fraction: 1.0,
        clarity: 1.0,
        permeation: BandGain::UNITY,
        filter: MuffleFilter::CLEAR,
    };

    /// Builds a response from traced visibility, discovered fraction, and permeation.
    pub(crate) fn new(
        direct_visible: bool,
        discovered_fraction: f32,
        permeation: BandGain,
    ) -> Self {
        let discovered_fraction = discovered_fraction.clamp(0.0, 1.0);
        let clarity = if direct_visible {
            1.0
        } else {
            1.0 - muffle_strength(discovered_fraction)
        };
        Self {
            direct_visible,
            discovered_fraction,
            clarity,
            permeation,
            filter: MuffleFilter::from_clarity_and_permeation(clarity, permeation),
        }
    }

    /// Returns whether the listener sees the source along a straight unobstructed line.
    #[must_use]
    pub const fn is_direct_visible(self) -> bool {
        self.direct_visible
    }

    /// Returns the energy-weighted fraction of primary rays that discovered the source.
    #[must_use]
    pub const fn discovered_fraction(self) -> f32 {
        self.discovered_fraction
    }

    /// Returns occlusion clarity, where `1` is unobstructed and `0` is fully hidden.
    #[must_use]
    pub const fn clarity(self) -> f32 {
        self.clarity
    }

    /// Returns `1 - clarity`, the muffle strength plotted against discovered rays.
    #[must_use]
    pub fn muffle_strength(self) -> f32 {
        1.0 - self.clarity
    }

    /// Returns mean amplitude transmission through walls by band.
    #[must_use]
    pub const fn permeation(self) -> BandGain {
        self.permeation
    }

    /// Returns the combined low- and high-frequency filter for this source.
    #[must_use]
    pub const fn filter(self) -> MuffleFilter {
        self.filter
    }
}

/// Maps the discovered-ray fraction to muffle strength on an exponential curve.
///
/// No discovered rays fully muffle a hidden source; about half of the rays makes it nearly clear.
#[must_use]
pub fn muffle_strength(discovered_fraction: f32) -> f32 {
    (-MUFFLE_CURVE_RATE * discovered_fraction.clamp(0.0, 1.0)).exp()
}

/// Listener reverb parameters estimated from echo and escaped rays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReverbEstimate {
    /// Fraction of bounce points with line of sight back to the listener, in `[0, 1]`.
    return_fraction: f32,
    /// Energy-weighted mean total length of returning echo paths, in meters.
    mean_return_distance_m: f32,
    /// Energy-weighted mean distance between successive bounces, in meters.
    mean_free_path_m: f32,
    /// Fraction of primary rays that escaped the scene, in `[0, 1]`.
    outdoor_fraction: f32,
    /// Estimated 60 dB decay time, in seconds.
    decay_time_s: f32,
    /// Low-band 60 dB decay time, in seconds.
    decay_low_s: f32,
    /// High-band 60 dB decay time, in seconds.
    decay_high_s: f32,
    /// First-reflection amplitude, including isolated outdoor reflections.
    early_gain: f32,
    /// Delay before early reflections arrive, in seconds.
    reflections_delay_s: f32,
    /// Suggested reverb send gain, in `[0, 1]`.
    wet_gain: f32,
}

impl ReverbEstimate {
    /// A dry estimate for an empty, fully outdoor scene.
    pub const DRY: Self = Self {
        return_fraction: 0.0,
        mean_return_distance_m: 0.0,
        mean_free_path_m: 0.0,
        outdoor_fraction: 1.0,
        decay_time_s: 0.0,
        decay_low_s: 0.0,
        decay_high_s: 0.0,
        early_gain: 0.0,
        reflections_delay_s: 0.0,
        wet_gain: 0.0,
    };

    /// Builds a manual estimate from a send gain, decay time, and early-reflection delay.
    ///
    /// Values are clamped to `[0, 1]`, `[0.05, 8]` seconds, and `[0, 0.3]` seconds. Non-finite
    /// values become the dry defaults. The remaining statistics report an enclosed space.
    #[must_use]
    pub fn from_parameters(wet_gain: f32, decay_time_s: f32, reflections_delay_s: f32) -> Self {
        let finite_or =
            |value: f32, fallback: f32| if value.is_finite() { value } else { fallback };
        Self {
            return_fraction: 1.0,
            outdoor_fraction: 0.0,
            wet_gain: finite_or(wet_gain, 0.0).clamp(0.0, 1.0),
            decay_time_s: finite_or(decay_time_s, narrow(MINIMUM_DECAY_S))
                .clamp(narrow(MINIMUM_DECAY_S), narrow(MAXIMUM_DECAY_S)),
            reflections_delay_s: finite_or(reflections_delay_s, 0.0)
                .clamp(0.0, narrow(MAXIMUM_REFLECTIONS_DELAY_S)),
            decay_low_s: finite_or(decay_time_s, 0.05).clamp(0.05, 8.0),
            decay_high_s: finite_or(decay_time_s * 0.6, 0.05).clamp(0.05, 8.0),
            early_gain: finite_or(wet_gain, 0.0).clamp(0.0, 1.0) * 0.35,
            ..Self::DRY
        }
    }

    /// Sets low- and high-band decay times for a manual reverb estimate.
    ///
    /// Values are bounded to 0.05–8 seconds; non-finite inputs use the mid-band decay.
    #[must_use]
    pub fn with_band_decay(mut self, low_s: f32, high_s: f32) -> Self {
        let bound = |value: f32| {
            if value.is_finite() {
                value.clamp(0.05, 8.0)
            } else {
                self.decay_time_s
            }
        };
        self.decay_low_s = bound(low_s);
        self.decay_high_s = bound(high_s);
        self
    }

    /// Builds an estimate from accumulated ray statistics.
    pub(crate) fn from_statistics(statistics: &EchoStatistics, ray_count: u32) -> Self {
        let rays = f64::from(ray_count.max(1));
        let outdoor_fraction = (f64::from(statistics.escaped_rays) / rays).clamp(0.0, 1.0);
        if statistics.bounce_count == 0 {
            return Self {
                outdoor_fraction: narrow(outdoor_fraction),
                ..Self::DRY
            };
        }

        let bounces = f64::from(statistics.bounce_count);
        let return_fraction = (f64::from(statistics.echo_count) / bounces).clamp(0.0, 1.0);
        let mean_return_distance_m = if statistics.echo_energy > 0.0 {
            statistics.echo_weighted_distance_m / statistics.echo_energy
        } else {
            0.0
        };
        let mean_free_path_m = statistics.bounce_leg_distance_m / bounces;
        // Escaped legs retain no energy, so open walls shorten the decay like absorption does.
        let legs = bounces + f64::from(statistics.escaped_rays);
        let decay = |reflected: f64| {
            let retention = (reflected / legs).clamp(1.0e-6, 0.999_999);
            (SIXTY_DECIBEL_LOG * mean_free_path_m / (SPEED_OF_SOUND_M_PER_S * -retention.ln()))
                .clamp(MINIMUM_DECAY_S, MAXIMUM_DECAY_S)
        };
        let decay_time_s = decay(statistics.reflected_energy_sum);
        // Early reflections arrive along first-order echo paths, not the long multi-bounce tail.
        let first_return_distance_m = if statistics.first_echo_energy > 0.0 {
            statistics.first_echo_weighted_distance_m / statistics.first_echo_energy
        } else {
            mean_return_distance_m
        };
        let reflections_delay_s = (first_return_distance_m / SPEED_OF_SOUND_M_PER_S)
            .clamp(0.0, MAXIMUM_REFLECTIONS_DELAY_S);
        // Visible return paths can carry no energy after an absorbing surface hit.
        // Count their surviving energy so an anechoic room cannot produce full-wet reverb.
        let return_energy = (statistics.echo_energy / bounces).clamp(0.0, 1.0);
        let wet_gain = (return_energy * (1.0 - outdoor_fraction))
            .sqrt()
            .clamp(0.0, 1.0);

        Self {
            return_fraction: narrow(return_fraction),
            mean_return_distance_m: narrow(mean_return_distance_m),
            mean_free_path_m: narrow(mean_free_path_m),
            outdoor_fraction: narrow(outdoor_fraction),
            decay_time_s: narrow(decay_time_s),
            decay_low_s: narrow(decay(statistics.reflected_low_sum)),
            decay_high_s: narrow(decay(statistics.reflected_high_sum)),
            // A single wall still produces an early return even when every ray eventually escapes.
            early_gain: narrow((statistics.first_echo_energy / rays).clamp(0.0, 1.0).sqrt() * 0.35),
            reflections_delay_s: narrow(reflections_delay_s),
            wet_gain: narrow(wet_gain),
        }
    }

    /// Returns the fraction of bounce points that could see the listener.
    #[must_use]
    pub const fn return_fraction(self) -> f32 {
        self.return_fraction
    }

    /// Returns the energy-weighted mean echo path length in meters.
    #[must_use]
    pub const fn mean_return_distance_m(self) -> f32 {
        self.mean_return_distance_m
    }

    /// Returns the mean distance between bounces in meters, a room-size estimate.
    #[must_use]
    pub const fn mean_free_path_m(self) -> f32 {
        self.mean_free_path_m
    }

    /// Returns the fraction of primary rays that escaped, where `1` is fully outdoors.
    #[must_use]
    pub const fn outdoor_fraction(self) -> f32 {
        self.outdoor_fraction
    }

    /// Returns the estimated 60 dB decay time in seconds.
    #[must_use]
    pub const fn decay_time_s(self) -> f32 {
        self.decay_time_s
    }

    /// Returns the low-frequency 60 dB decay time in seconds.
    #[must_use]
    pub const fn decay_low_s(self) -> f32 {
        self.decay_low_s
    }

    /// Returns the high-frequency 60 dB decay time in seconds.
    #[must_use]
    pub const fn decay_high_s(self) -> f32 {
        self.decay_high_s
    }

    /// Returns the early-reflection send, which can be nonzero outdoors near surfaces.
    #[must_use]
    pub const fn early_gain(self) -> f32 {
        self.early_gain
    }

    /// Returns the estimated early-reflection delay in seconds.
    #[must_use]
    pub const fn reflections_delay_s(self) -> f32 {
        self.reflections_delay_s
    }

    /// Returns the suggested reverb send gain.
    #[must_use]
    pub const fn wet_gain(self) -> f32 {
        self.wet_gain
    }
}

impl Default for ReverbEstimate {
    /// Returns [`ReverbEstimate::DRY`].
    fn default() -> Self {
        Self::DRY
    }
}

/// Running echo, bounce, and escape totals accumulated during one trace.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EchoStatistics {
    /// Number of surface hits across all primary rays.
    pub(crate) bounce_count: u32,
    /// Number of hits with line of sight back to the listener.
    pub(crate) echo_count: u32,
    /// Number of primary rays that escaped.
    pub(crate) escaped_rays: u32,
    /// Sum of leg lengths ending in a surface hit, in meters.
    pub(crate) bounce_leg_distance_m: f64,
    /// Sum of per-hit mid-band reflected energy fractions.
    pub(crate) reflected_energy_sum: f64,
    /// Sum of per-hit low-band reflected energy fractions.
    pub(crate) reflected_low_sum: f64,
    /// Sum of per-hit high-band reflected energy fractions.
    pub(crate) reflected_high_sum: f64,
    /// Sum of mid-band energy carried by echo rays.
    pub(crate) echo_energy: f64,
    /// Sum of echo energy multiplied by echo path length.
    pub(crate) echo_weighted_distance_m: f64,
    /// Sum of mid-band energy carried by first-bounce echo rays.
    pub(crate) first_echo_energy: f64,
    /// Sum of first-bounce echo energy multiplied by echo path length.
    pub(crate) first_echo_weighted_distance_m: f64,
}

/// Running per-source discovery and permeation totals.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SourceAccumulator {
    /// Whether the listener sees the source directly.
    pub(crate) direct_visible: bool,
    /// Sum of ray energy at the first bounce that discovered the source.
    pub(crate) discovered_energy: f64,
    /// Number of permeation rays cast.
    pub(crate) permeation_rays: u32,
    /// Summed low-band permeation amplitude.
    pub(crate) permeation_low: f64,
    /// Summed mid-band permeation amplitude.
    pub(crate) permeation_mid: f64,
    /// Summed high-band permeation amplitude.
    pub(crate) permeation_high: f64,
}

impl SourceAccumulator {
    /// Adds one permeation ray's band amplitudes.
    pub(crate) fn add_permeation(&mut self, gain: BandGain, weight: f64) {
        self.permeation_rays += 1;
        self.permeation_low = f64::from(gain.low()).mul_add(weight, self.permeation_low);
        self.permeation_mid = f64::from(gain.mid()).mul_add(weight, self.permeation_mid);
        self.permeation_high = f64::from(gain.high()).mul_add(weight, self.permeation_high);
    }

    /// Finishes the accumulator into a public response.
    pub(crate) fn finish(self, ray_count: u32) -> SourceRayResponse {
        let discovered = narrow(self.discovered_energy / f64::from(ray_count.max(1)));
        let permeation = if self.permeation_rays == 0 {
            BandGain::ZERO
        } else {
            let rays = f64::from(self.permeation_rays);
            BandGain::try_new(
                narrow((self.permeation_low / rays).clamp(0.0, 1.0)),
                narrow((self.permeation_mid / rays).clamp(0.0, 1.0)),
                narrow((self.permeation_high / rays).clamp(0.0, 1.0)),
            )
            .unwrap_or(BandGain::ZERO)
        };
        SourceRayResponse::new(self.direct_visible, discovered, permeation)
    }
}

/// A small deterministic generator for direction jitter and diffuse scattering.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TraceRng {
    /// Current xorshift state; never zero.
    state: u64,
}

impl TraceRng {
    /// Seeds the generator, replacing a zero seed with a fixed nonzero value.
    pub(crate) const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Returns a uniformly distributed value in `[0, 1)`.
    pub(crate) fn next_unit(&mut self) -> f64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        // The top 32 bits are exactly representable and give ample resolution for directions.
        f64::from(u32::try_from(value >> 32).unwrap_or(u32::MAX)) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    //! Mapping and validation checks for shared ray-trace values.

    use super::{MuffleFilter, RayTraceSettings, SourceRayResponse, TraceRng, muffle_strength};
    use crate::BandGain;

    /// Discovering no rays fully muffles; a quarter of rays is nearly clear.
    #[test]
    fn muffle_curve_falls_quickly() {
        assert!((muffle_strength(0.0) - 1.0).abs() < f32::EPSILON);
        assert!(muffle_strength(0.5) < 0.06);
        assert!(muffle_strength(0.1) > muffle_strength(0.2));
    }

    /// Direct visibility is always clear, while a hidden source loses treble before bass.
    #[test]
    fn hidden_sources_lose_high_frequencies_first() {
        let visible = SourceRayResponse::new(true, 0.0, BandGain::ZERO);
        assert_eq!(visible.filter(), MuffleFilter::CLEAR);

        let hidden = SourceRayResponse::new(false, 0.08, BandGain::ZERO);
        assert!(hidden.filter().gain_hf() < hidden.filter().gain_lf());
        assert!(hidden.filter().gain_lf() < 1.0);

        let leaking =
            SourceRayResponse::new(false, 0.0, BandGain::try_new(0.4, 0.2, 0.05).unwrap());
        assert!((leaking.filter().gain_lf() - 0.4).abs() < 1.0e-6);
        assert!((leaking.filter().gain_hf() - 0.05).abs() < 1.0e-6);
    }

    /// Builders reject invalid counts and coefficients.
    #[test]
    fn settings_validate_inputs() {
        assert!(RayTraceSettings::default().try_with_ray_count(0).is_err());
        assert!(
            RayTraceSettings::default()
                .try_with_scattering(1.5)
                .is_err()
        );
        assert!(
            RayTraceSettings::default()
                .try_with_escape_distance(0.0)
                .is_err()
        );
        assert!(
            RayTraceSettings::default()
                .try_with_escape_distance(f64::NAN)
                .is_err()
        );
        assert!(MuffleFilter::try_new(0.5, 1.5).is_err());
    }

    /// The generator stays in the unit interval and is deterministic.
    #[test]
    fn rng_is_deterministic_and_bounded() {
        let mut first = TraceRng::new(7);
        let mut second = TraceRng::new(7);
        for _ in 0..1000 {
            let value = first.next_unit();
            assert!((0.0..1.0).contains(&value));
            assert_eq!(value.to_bits(), second.next_unit().to_bits());
        }
        assert!(TraceRng::new(0).next_unit() > 0.0);
    }
}
