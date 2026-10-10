//! Identical deterministic acoustic workloads for Criterion, profilers and real browsers.

#![expect(
    clippy::suboptimal_flops,
    reason = "fixture arithmetic is shared verbatim with baseline WebAssembly without software FMA calls"
)]

use bevy_raytraced_audio::{
    AcousticDspParams, AcousticDspProcessor, AcousticMaterial, AcousticScene2d, AcousticScene3d,
    BandAbsorption, BandGain, BinauralParams, BinauralProcessor, Emitter2d, Emitter3d, Listener2d,
    Listener3d, ListenerTrace2d, ListenerTrace3d, MuffleFilter, Point2, Point3, RayTraceSettings,
    Segment2d, Triangle3d,
};
use std::{hint::black_box, sync::Arc};
mod cache;
pub use cache::{CACHE_CASES, CacheWorkload, cache_case_names};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Frozen benchmark inventory. Every optimization round must measure every entry.
pub const CASES: &[&str] = &[
    "listener/2d/1",
    "listener/2d/64",
    "listener/2d/1024",
    "listener/3d/1",
    "listener/3d/64",
    "listener/3d/1024",
    "listener/2d/recorded",
    "listener/3d/recorded",
    "churn/2d/128",
    "churn/3d/128",
    "churn/2d/1024",
    "churn/3d/1024",
    "dsp/44100/mono",
    "dsp/48000/stereo",
    "dsp/48000/changing64",
    "hrtf/44100/static",
    "hrtf/48000/static",
    "hrtf/96000/static",
    "hrtf/44100/moving",
    "hrtf/48000/moving",
    "hrtf/48000/bypass",
];

/// Converts bounded fixture indices to exact floating point values.
fn number(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("bounded fixture"))
}
/// A validated point in a deterministic fixture.
fn p2(x: f32, y: f32) -> Point2 {
    Point2::try_new(x, y).expect("finite fixture")
}
/// A validated point in a deterministic fixture.
fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3::try_new(x, y, z).expect("finite fixture")
}
/// Moderately reflective, permeable material exercises all ray kinds.
fn material() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.25, 0.5).expect("fixture"))
        .try_with_transmission(BandGain::try_new(0.2, 0.1, 0.05).expect("fixture"))
        .expect("conserved fixture")
}
/// Replaces every 2D surface, including the enclosing room, with shifted geometry.
fn scene2(scene: &mut AcousticScene2d, count: usize, phase: f32) {
    scene.clear();
    for (a, b) in [
        ((-10., -10.), (10., -10.)),
        ((10., -10.), (10., 10.)),
        ((10., 10.), (-10., 10.)),
        ((-10., 10.), (-10., -10.)),
    ] {
        scene
            .add_segment(Segment2d::try_new(p2(a.0, a.1), p2(b.0, b.1), material()).expect("wall"));
    }
    for i in 0..count {
        let x = number(i % 32) * 0.5 - 8. + phase;
        let y = number(i / 32) * 0.5 - 8.;
        scene.add_segment(Segment2d::try_new(p2(x, y), p2(x, y + 0.35), material()).expect("wall"));
    }
}
/// Replaces every 3D triangle with an enclosed room and shifted interior baffles.
fn scene3(scene: &mut AcousticScene3d, count: usize, phase: f32) {
    scene.clear();
    for [a, b, c, d] in [
        [
            [-10., -10., -10.],
            [10., -10., -10.],
            [10., 10., -10.],
            [-10., 10., -10.],
        ],
        [
            [-10., -10., 10.],
            [10., -10., 10.],
            [10., 10., 10.],
            [-10., 10., 10.],
        ],
        [
            [-10., -10., -10.],
            [-10., 10., -10.],
            [-10., 10., 10.],
            [-10., -10., 10.],
        ],
        [
            [10., -10., -10.],
            [10., 10., -10.],
            [10., 10., 10.],
            [10., -10., 10.],
        ],
        [
            [-10., -10., -10.],
            [10., -10., -10.],
            [10., -10., 10.],
            [-10., -10., 10.],
        ],
        [
            [-10., 10., -10.],
            [10., 10., -10.],
            [10., 10., 10.],
            [-10., 10., 10.],
        ],
    ] {
        for vertices in [[a, b, c], [a, c, d]] {
            scene.add_triangle(
                Triangle3d::try_new(vertices.map(|[x, y, z]| p3(x, y, z)), material())
                    .expect("wall"),
            );
        }
    }
    for i in 0..count {
        let x = number(i % 32) * 0.5 - 8. + phase;
        let z = number(i / 32) * 0.5 - 8.;
        scene.add_triangle(
            Triangle3d::try_new(
                [p3(x, -2., z), p3(x, 2., z), p3(x, 0., z + 0.35)],
                material(),
            )
            .expect("baffle"),
        );
    }
}
/// A warmed workload; construction is deliberately outside measured iterations.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Workload {
    /// Stateful production operations and their checksum.
    run: Box<dyn FnMut() -> f64>,
}
impl core::fmt::Debug for Workload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Workload").finish_non_exhaustive()
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl Workload {
    /// Builds one case from the frozen inventory.
    ///
    /// # Panics
    /// Panics for an unknown name or invalid internal fixture.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "frozen inventory keeps each complete workload together"
    )]
    pub fn new(name: &str) -> Self {
        assert!(CASES.contains(&name), "unknown workload: {name}");
        let mut fields = name.split('/');
        let kind = fields.next().expect("kind");
        let configuration = fields.next().expect("configuration");
        let variant = fields.next().expect("variant");
        let mut run: Box<dyn FnMut() -> f64> = if kind == "listener" || kind == "churn" {
            let changing = kind == "churn";
            let recorded = variant == "recorded";
            let count = variant.parse().unwrap_or(64);
            let surfaces = if changing { count } else { 256 };
            let mut frame = 0_usize;
            let settings = RayTraceSettings::default()
                .try_with_ray_count(64)
                .expect("rays")
                .with_max_bounces(4)
                .with_permeation_ray_count(8)
                .with_recorded_rays(recorded);
            if configuration == "2d" {
                let mut scene = AcousticScene2d::default();
                scene2(&mut scene, surfaces, 0.);
                let mut sources: Vec<_> = (0..count)
                    .map(|i| {
                        Emitter2d::new(p2(number(i % 32) * 0.4 - 6., number(i / 32) * 0.4 - 6.))
                    })
                    .collect();
                let mut output = ListenerTrace2d::default();
                Box::new(move || {
                    if changing {
                        frame = (frame + 1) % 32;
                        let shift = number(frame) * 0.002;
                        scene2(&mut scene, surfaces, shift);
                        for (i, source) in sources.iter_mut().enumerate() {
                            *source = Emitter2d::new(p2(
                                number(i % 32) * 0.4 - 6. + shift,
                                number(i / 32) * 0.4 - 6.,
                            ));
                        }
                    }
                    scene.trace_listener(
                        Listener2d::new(p2(0.2, 0.3)),
                        black_box(&sources),
                        settings,
                        &mut output,
                    );
                    black_box(&output);
                    f64::from(output.reverb().outdoor_fraction())
                        + output
                            .sources()
                            .iter()
                            .map(|s| f64::from(s.filter().gain_hf()))
                            .sum::<f64>()
                })
            } else {
                let mut scene = AcousticScene3d::default();
                scene3(&mut scene, surfaces, 0.);
                let mut sources: Vec<_> = (0..count)
                    .map(|i| {
                        Emitter3d::new(p3(
                            number(i % 32) * 0.4 - 6.,
                            0.5,
                            number(i / 32) * 0.4 - 6.,
                        ))
                    })
                    .collect();
                let mut output = ListenerTrace3d::default();
                Box::new(move || {
                    if changing {
                        frame = (frame + 1) % 32;
                        let shift = number(frame) * 0.002;
                        scene3(&mut scene, surfaces, shift);
                        for (i, source) in sources.iter_mut().enumerate() {
                            *source = Emitter3d::new(p3(
                                number(i % 32) * 0.4 - 6. + shift,
                                0.5,
                                number(i / 32) * 0.4 - 6.,
                            ));
                        }
                    }
                    scene.trace_listener(
                        Listener3d::new(p3(0.2, 0.3, 0.4)),
                        black_box(&sources),
                        settings,
                        &mut output,
                    );
                    black_box(&output);
                    f64::from(output.reverb().outdoor_fraction())
                        + output
                            .sources()
                            .iter()
                            .map(|s| f64::from(s.filter().gain_hf()))
                            .sum::<f64>()
                })
            }
        } else if kind == "dsp" {
            let rate = configuration.parse().expect("rate");
            let voices = if variant == "changing64" { 64 } else { 1 };
            let channels = if variant == "stereo" { 2 } else { 1 };
            let controls: Vec<_> = (0..voices)
                .map(|_| Arc::new(AcousticDspParams::default()))
                .collect();
            let mut room = AcousticScene3d::default();
            scene3(&mut room, 0, 0.);
            let mut trace = ListenerTrace3d::default();
            room.trace_listener(
                Listener3d::new(p3(0., 0., 0.)),
                &[],
                RayTraceSettings::default(),
                &mut trace,
            );
            for p in &controls {
                p.set_reverb(trace.reverb(), 0.5);
            }
            let mut processors: Vec<_> = controls
                .iter()
                .map(|p| AcousticDspProcessor::new(p.clone(), channels, rate))
                .collect();
            let mut frame = 0_usize;
            Box::new(move || {
                frame = (frame + 1) % 32;
                if voices > 1 {
                    for p in &controls {
                        p.set_filter(MuffleFilter::from_clarity_and_permeation(
                            number(frame) / 31.,
                            BandGain::ZERO,
                        ));
                    }
                }
                let mut sum = 0.;
                for processor in &mut processors {
                    for i in 0..256 {
                        sum += f64::from(
                            processor.process_sample(black_box(number(i % 17) * 0.01 - 0.08)),
                        );
                    }
                }
                black_box(sum)
            })
        } else {
            let rate = configuration.parse().expect("rate");
            let params = Arc::new(BinauralParams::new(1.));
            params.set_position([1., 0.5, -1.]);
            params.set_enabled(variant != "bypass");
            let mut processor = BinauralProcessor::new(params.clone(), rate);
            let moving = variant == "moving";
            let mut frame = 0_usize;
            Box::new(move || {
                frame = (frame + 1) % 32;
                if moving {
                    params.set_position([number(frame) * 0.1 - 1.5, 0.5, -1.]);
                }
                let mut sum = 0.;
                for i in 0..256 {
                    let out =
                        processor.process_frame(black_box(number(i % 17) * 0.01 - 0.08), 0.01);
                    sum += f64::from(out[0] + out[1]);
                }
                black_box(sum)
            })
        };
        black_box(run());
        Self { run }
    }
    /// Performs a fixed number of iterations and returns a consumed finite checksum.
    pub fn run(&mut self, iterations: u32) -> f64 {
        (0..iterations).map(|_| black_box((self.run)())).sum()
    }
}
/// Lists the exact case names for browser runners.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[must_use]
pub fn case_names() -> String {
    CASES.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    /// All workloads execute production code and remain finite under repeated mutation.
    #[test]
    fn stress_workloads_are_finite() {
        for name in CASES {
            assert!(Workload::new(name).run(3).is_finite(), "{name}");
        }
    }
}
