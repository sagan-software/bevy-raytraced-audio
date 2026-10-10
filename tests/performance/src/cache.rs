//! Matched cached/uncached traces with deterministic movement and emitter churn.

use super::{number, p2, p3, scene2, scene3};
use bevy_raytraced_audio::{
    AcousticScene2d, AcousticScene3d, Emitter2d, Emitter3d, Listener2d, Listener3d,
    ListenerTrace2d, ListenerTrace3d, RayTraceSettings, TraceCacheStatistics,
};
use std::hint::black_box;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// A fixed inventory; cache mode is a separate parameter, never a different workload.
pub const CACHE_CASES: &[&str] = &[
    "2d/static",
    "3d/static",
    "2d/listener",
    "3d/listener",
    "2d/joint",
    "3d/joint",
    "2d/last_source",
    "3d/last_source",
    "2d/spawn",
    "3d/spawn",
    "2d/geometry",
    "3d/geometry",
    "2d/active",
    "3d/active",
    "2d/mixed_idle",
    "3d/mixed_idle",
];

/// One acoustic result and its accumulated cache decisions.
#[derive(Clone, Copy, Debug, Default)]
struct Step {
    /// Finite value consumed by the benchmark to retain computation.
    checksum: f64,
    /// Counters from the production listener-trace output.
    statistics: TraceCacheStatistics,
}

/// A changing workload whose input sequence is identical with either cache policy.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct CacheWorkload {
    /// Stateful production operation, including the actual scene mutation costs.
    step: Box<dyn FnMut() -> Step>,
    /// Last completed operation's diagnostics.
    latest: Step,
}

impl core::fmt::Debug for CacheWorkload {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CacheWorkload")
            .field("latest", &self.latest)
            .finish_non_exhaustive()
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl CacheWorkload {
    /// Constructs a deterministic 64-source workload, or 1,024 for late source movement.
    ///
    /// # Panics
    /// Panics for an unknown case or invalid internal fixture.
    #[must_use]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(name: &str, reuse_results: bool) -> Self {
        assert!(CACHE_CASES.contains(&name), "unknown cache case");
        let (dimension, scenario) = name.split_once('/').expect("case format");
        let scenario = scenario.to_owned();
        let count = if scenario == "last_source" { 1024 } else { 64 };
        let settings = RayTraceSettings::default()
            .try_with_ray_count(64)
            .expect("rays")
            .with_max_bounces(4)
            .with_permeation_ray_count(8)
            .with_result_reuse(reuse_results);
        let mut frame = 0_usize;
        let step: Box<dyn FnMut() -> Step> = if dimension == "2d" {
            let mut scene = AcousticScene2d::default();
            scene2(&mut scene, 128, 0.0);
            let mut sources: Vec<_> = (0..count)
                .map(|i| Emitter2d::new(p2(number(i % 32) * 0.4 - 6., number(i / 32) * 0.4 - 6.)))
                .collect();
            let mut listener = Listener2d::new(p2(0., 0.));
            let mut output = ListenerTrace2d::default();
            Box::new(move || {
                frame = (frame + 1) % 1024;
                let shift = number(frame) * 0.0005;
                if matches!(
                    scenario.as_str(),
                    "listener" | "joint" | "geometry" | "active"
                ) || (scenario == "mixed_idle" && frame.is_multiple_of(10))
                {
                    listener = Listener2d::new(p2(shift, -shift));
                }
                if matches!(scenario.as_str(), "joint" | "geometry" | "active") {
                    for (i, source) in sources.iter_mut().enumerate() {
                        *source = Emitter2d::new(p2(
                            number(i % 32) * 0.4 - 6. + shift,
                            number(i / 32) * 0.4 - 6.,
                        ));
                    }
                }
                if scenario == "last_source" {
                    *sources.last_mut().expect("sources") = Emitter2d::new(p2(shift, 1.));
                }
                if matches!(scenario.as_str(), "spawn" | "active") {
                    sources.truncate(count);
                    for i in 0..frame % 4 {
                        sources.push(Emitter2d::new(p2(shift, number(i))));
                    }
                }
                if matches!(scenario.as_str(), "geometry" | "active") {
                    scene2(&mut scene, 128, shift);
                }
                scene.trace_listener(listener, &sources, settings, &mut output);
                Step {
                    checksum: output
                        .sources()
                        .iter()
                        .map(|r| f64::from(r.filter().gain_lf()) + f64::from(r.filter().gain_hf()))
                        .sum::<f64>()
                        + f64::from(output.reverb().wet_gain()),
                    statistics: output.cache_statistics(),
                }
            })
        } else {
            let mut scene = AcousticScene3d::default();
            scene3(&mut scene, 128, 0.0);
            let mut sources: Vec<_> = (0..count)
                .map(|i| {
                    Emitter3d::new(p3(number(i % 32) * 0.4 - 6., 0., number(i / 32) * 0.4 - 6.))
                })
                .collect();
            let mut listener = Listener3d::new(p3(0., 0., 0.));
            let mut output = ListenerTrace3d::default();
            Box::new(move || {
                frame = (frame + 1) % 1024;
                let shift = number(frame) * 0.0005;
                if matches!(
                    scenario.as_str(),
                    "listener" | "joint" | "geometry" | "active"
                ) || (scenario == "mixed_idle" && frame.is_multiple_of(10))
                {
                    listener = Listener3d::new(p3(shift, 0., -shift));
                }
                if matches!(scenario.as_str(), "joint" | "geometry" | "active") {
                    for (i, source) in sources.iter_mut().enumerate() {
                        *source = Emitter3d::new(p3(
                            number(i % 32) * 0.4 - 6. + shift,
                            0.,
                            number(i / 32) * 0.4 - 6.,
                        ));
                    }
                }
                if scenario == "last_source" {
                    *sources.last_mut().expect("sources") = Emitter3d::new(p3(shift, 0., 1.));
                }
                if matches!(scenario.as_str(), "spawn" | "active") {
                    sources.truncate(count);
                    for i in 0..frame % 4 {
                        sources.push(Emitter3d::new(p3(shift, 0., number(i))));
                    }
                }
                if matches!(scenario.as_str(), "geometry" | "active") {
                    scene3(&mut scene, 128, shift);
                }
                scene.trace_listener(listener, &sources, settings, &mut output);
                Step {
                    checksum: output
                        .sources()
                        .iter()
                        .map(|r| f64::from(r.filter().gain_lf()) + f64::from(r.filter().gain_hf()))
                        .sum::<f64>()
                        + f64::from(output.reverb().wet_gain()),
                    statistics: output.cache_statistics(),
                }
            })
        };
        Self {
            step,
            latest: Step::default(),
        }
    }

    /// Runs the next deterministic frames, retaining a finite acoustic checksum.
    pub fn run(&mut self, iterations: u32) -> f64 {
        let mut checksum = 0.0;
        for _ in 0..iterations {
            self.latest = (self.step)();
            checksum += black_box(self.latest.checksum);
        }
        checksum
    }

    /// Returns diagnostics as JSON, also readable from real browser benchmarks.
    #[must_use]
    pub fn statistics_json(&self) -> String {
        let s = self.latest.statistics;
        format!(
            "{{\"hits\":{},\"computations\":{},\"forced\":{},\"cold\":{},\"scene_changes\":{},\"listener_changes\":{},\"settings_changes\":{},\"source_changes\":{}}}",
            s.hits,
            s.computations,
            s.forced,
            s.cold,
            s.scene_changes,
            s.listener_changes,
            s.settings_changes,
            s.source_changes
        )
    }
}

/// Returns the frozen cache workload names for the browser harness.
#[must_use]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cache_case_names() -> String {
    CACHE_CASES.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{CACHE_CASES, CacheWorkload};

    /// All moving/churning inputs preserve the same result with the cache disabled.
    #[test]
    fn cache_policies_match_for_every_mutation_sequence() {
        for name in CACHE_CASES {
            let mut cached = CacheWorkload::new(name, true);
            let mut uncached = CacheWorkload::new(name, false);
            for frame in 0..24 {
                assert_eq!(cached.run(1), uncached.run(1), "{name}, frame {frame}");
            }
            assert_eq!(uncached.latest.statistics.forced, 24);
            assert_eq!(uncached.latest.statistics.hits, 0);
            let stats = cached.latest.statistics;
            if name.ends_with("static") {
                assert_eq!(stats.hits, 23);
            } else if name.ends_with("mixed_idle") {
                assert_eq!(stats.hits, 21);
            } else {
                assert_eq!(stats.hits, 0);
            }
        }
    }
}
