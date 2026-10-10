//! Opt-in, full-application measurements shared by the three rendered workloads.

#![expect(
    clippy::indexing_slicing,
    reason = "JSON field reads return Null when absent; writes target an initialized JSON object"
)]
use bevy::{
    audio::Volume,
    platform::time::Instant,
    prelude::*,
    render::{Render, RenderApp, RenderSystems, renderer::RenderAdapterInfo},
    window::{PresentMode, PrimaryWindow},
};
use bevy_raytraced_audio::{ListenerTrace2d, ListenerTrace3d, TraceCacheStatistics};
use bevy_raytraced_audio_2d::{
    RaytracedAudioEmitter2d, RaytracedAudioListener2d, RaytracedAudioListenerTrace2d,
    RaytracedAudioSurface2d, RaytracedAudioTracing2d,
};
use bevy_raytraced_audio_3d::{
    RaytracedAudioEmitter3d, RaytracedAudioListener3d, RaytracedAudioListenerTrace3d,
    RaytracedAudioPlayer, RaytracedAudioSurface3d, RaytracedAudioTracing3d,
};
use serde_json::{Value, json};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU32, Ordering},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "export function frame_config() { window.acousticFrameHidden = document.hidden; document.addEventListener('visibilitychange', () => { window.acousticFrameHidden ||= document.hidden; }); return new URLSearchParams(location.search).get('frame_bench') || ''; } export function frame_result(s) { window.acousticFrameReport = {...JSON.parse(s), userAgent: navigator.userAgent, devicePixelRatio, hidden: window.acousticFrameHidden || document.hidden, audioContexts: [...(window.__bevyAudioContexts || [])].map(c => ({state:c.state, sampleRate:c.sampleRate}))}; }"
)]
extern "C" {
    /// Reads the same JSON configuration as the native environment variable.
    fn frame_config() -> String;
    /// Publishes the complete report for browser automation and downloads.
    fn frame_result(report: &str);
}

/// Enables benchmarking only when explicit JSON configuration is provided.
#[derive(Debug, Clone, Copy)]
pub(super) struct FrameBenchmarkPlugin(pub(super) &'static str);

/// Counts completed render schedules, independently of application updates.
#[derive(Resource, Clone, Debug, Default)]
struct RenderCount {
    /// Renderer iterations, not inferred from application update count.
    frames: Arc<AtomicU32>,
    /// The driver actually selected by WGPU, recorded from the render world.
    gpu: Arc<OnceLock<String>>,
}

/// State and unaggregated wall-clock measurements for one opt-in run.
#[derive(Resource)]
pub(super) struct FrameBenchmark {
    /// Configuration is preserved verbatim in the report.
    config: Value,
    /// Application whose unchanged visual assets are rendered.
    name: &'static str,
    /// Wall clock used by the reproducible movement driver.
    started: Instant,
    /// Beginning of the preceding main application frame.
    previous: Instant,
    /// Beginning of the current CPU update.
    frame_start: Instant,
    /// Excluded startup and shader/audio warm-up duration.
    warmup: f64,
    /// Requested sampling duration, excluding warm-up.
    duration: f64,
    /// Full frame intervals, including render scheduling and presentation backpressure.
    frame_ms: Vec<f64>,
    /// Time in the main schedule, excluding asynchronous render work.
    update_ms: Vec<f64>,
    /// Trace diagnostics at the beginning of the measurement window.
    baseline: Option<[u64; 12]>,
    /// Completed render schedules at the start of the measurement window.
    render_start: u32,
    /// Last processed wall-clock sound-spawn epoch.
    epoch: u32,
    /// Spawn/despawn operations during the entire run, including warm-up.
    churn: [u32; 2],
    /// Prevents browser reports from being rewritten after completion.
    finished: bool,
    /// Minimum and maximum live processed sinks during the measured window.
    sink_bounds: [usize; 2],
    /// Optional sampling, deliberately separate from accepted timing runs.
    #[cfg(all(feature = "frame-profile", target_os = "linux"))]
    profiler: Option<pprof::ProfilerGuard<'static>>,
}

impl FrameBenchmark {
    /// Wall-clock animation time, identical for both cache policies.
    pub(super) fn seconds(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }
}

impl Plugin for FrameBenchmarkPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        let input = std::env::var("ACOUSTIC_FRAME_BENCH").unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let input = frame_config();
        if input.is_empty() {
            return;
        }
        let config: Value = serde_json::from_str(&input).expect("frame benchmark JSON");
        let warmup = config["warmup_s"].as_f64().unwrap_or(15.0);
        let duration = config["duration_s"].as_f64().unwrap_or(20.0);
        assert!(warmup.is_finite() && warmup >= 0.0 && duration.is_finite() && duration > 0.0);
        let now = Instant::now();
        let counter = RenderCount::default();
        app.insert_resource(counter.clone())
            .insert_resource(FrameBenchmark {
                config,
                name: self.0,
                started: now,
                previous: now,
                frame_start: now,
                warmup,
                duration,
                frame_ms: Vec::new(),
                update_ms: Vec::new(),
                baseline: None,
                render_start: 0,
                epoch: u32::MAX,
                churn: [0, 0],
                finished: false,
                sink_bounds: [usize::MAX, 0],
                #[cfg(all(feature = "frame-profile", target_os = "linux"))]
                profiler: None,
            })
            .add_systems(PostStartup, configure)
            .add_systems(First, begin_frame)
            .add_systems(Update, drive_scene)
            .add_systems(Last, end_frame);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(counter)
                .add_systems(Render, count_render.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Tracks completed renderer schedules rather than inferring them from the FPS HUD.
fn count_render(count: Res<'_, RenderCount>, adapter: Res<'_, RenderAdapterInfo>) {
    count.gpu.get_or_init(|| format!("{:?}", **adapter));
    count.frames.fetch_add(1, Ordering::Relaxed);
}

/// A benchmark source has a stable phase and may be replaced every second.
#[derive(Component)]
struct BenchSource {
    /// Deterministic angular spacing.
    phase: f32,
    /// Short-lived voices exercise creation and cleanup.
    transient: bool,
}

/// Adds one source with an actual processed voice when requested.
fn spawn_source(world: &mut World, dimension2: bool, phase: f32, transient: bool, voice: bool) {
    let audio: Handle<AudioSource> = world.resource::<AssetServer>().load("audio/voice.ogg");
    let mut entity = world.spawn((
        BenchSource { phase, transient },
        Transform::from_xyz(3.0, 0.3, 3.0),
    ));
    if dimension2 {
        entity.insert(RaytracedAudioEmitter2d);
    } else {
        entity.insert(RaytracedAudioEmitter3d);
    }
    if voice {
        entity.insert((
            RaytracedAudioPlayer::new(audio)
                .with_reverb_send(0.65)
                .with_binaural(0.22),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.015)),
        ));
    }
}

/// Makes the benchmark policy explicit while retaining the application's ray quality.
fn configure(world: &mut World) {
    let state = world.resource::<FrameBenchmark>();
    let cache = state.config["cache"].as_bool().unwrap_or(false);
    let interval = state.config["interval_s"].as_f64().unwrap_or(0.0);
    let dimension2 = state.name == "stress_2d";
    let stress = state.name.starts_with("stress");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded finite configuration converted to Bevy f32 interval"
    )]
    let interval = interval.clamp(0.0, 10.0) as f32;
    if let Some(mut tracing) = world.get_resource_mut::<RaytracedAudioTracing2d>() {
        tracing.settings = tracing.settings.with_result_reuse(cache);
        tracing.interval_s = interval;
    }
    if let Some(mut tracing) = world.get_resource_mut::<RaytracedAudioTracing3d>() {
        tracing.settings = tracing.settings.with_result_reuse(cache);
        tracing.interval_s = interval;
    }
    for mut window in world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .iter_mut(world)
    {
        window.present_mode = PresentMode::AutoNoVsync;
    }
    // The original stress scenes have 16 visible sources. The benchmark adds 240 traced
    // sources and four processed voices; short-lived voices are added by the scene driver.
    if stress {
        for index in 0..240_u16 {
            spawn_source(world, dimension2, f32::from(index) * 0.17, false, index < 4);
        }
    }
    let mut state = world.resource_mut::<FrameBenchmark>();
    state.started = Instant::now();
    state.previous = state.started;
}

/// Records full frame intervals only after startup has warmed all application systems.
fn begin_frame(world: &mut World) {
    let now = Instant::now();
    let diagnostics = statistics(world);
    let rendered = world
        .resource::<RenderCount>()
        .frames
        .load(Ordering::Relaxed);
    let mut state = world.resource_mut::<FrameBenchmark>();
    state.frame_start = now;
    if state.finished {
        return;
    }
    if now.duration_since(state.started).as_secs_f64() >= state.warmup {
        if state.baseline.is_none() {
            state.baseline = Some(diagnostics);
            #[cfg(not(target_arch = "wasm32"))]
            println!("ACOUSTIC_MEASUREMENT_STARTED");
            state.render_start = rendered;
            #[cfg(all(feature = "frame-profile", target_os = "linux"))]
            if state.config["flamegraph"].as_str().is_some() {
                state.profiler = Some(
                    pprof::ProfilerGuardBuilder::default()
                        .frequency(997)
                        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
                        .build()
                        .expect("sampling profiler"),
                );
            }
        } else {
            let elapsed = now.duration_since(state.previous).as_secs_f64() * 1000.0;
            state.frame_ms.push(elapsed);
        }
    }
    state.previous = now;
}

/// Moves sources and stress listeners, edits geometry, and replaces four voices each second.
fn drive_scene(world: &mut World) {
    let state = world.resource::<FrameBenchmark>();
    if state.finished {
        return;
    }
    let seconds = state.seconds();
    let epoch =
        u32::try_from(state.started.elapsed().as_secs()).expect("bounded benchmark duration");
    let dimension2 = state.name == "stress_2d";
    let stress = state.name.starts_with("stress");
    let replace = state.epoch != epoch;
    if replace {
        let old: Vec<_> = world
            .query::<(Entity, &BenchSource)>()
            .iter(world)
            .filter_map(|(e, s)| s.transient.then_some(e))
            .collect();
        let removed = u32::try_from(old.len()).expect("four transient voices");
        for entity in old {
            world.despawn(entity);
        }
        for index in 0..4_u16 {
            spawn_source(world, dimension2, f32::from(index) * 1.5, true, true);
        }
        let mut state = world.resource_mut::<FrameBenchmark>();
        state.epoch = epoch;
        state.churn[0] += 4;
        state.churn[1] += removed;
    }
    for (mut transform, source) in world
        .query::<(&mut Transform, &BenchSource)>()
        .iter_mut(world)
    {
        let angle = seconds.mul_add(0.3, source.phase);
        let (sin, cos) = angle.sin_cos();
        transform.translation = if dimension2 {
            Vec3::new(4.0 * cos, 4.0 * sin, 1.0)
        } else {
            Vec3::new(4.0 * cos, 0.7, 4.0 * sin)
        };
    }
    if stress {
        for mut transform in world
            .query_filtered::<&mut Transform, With<RaytracedAudioListener2d>>()
            .iter_mut(world)
        {
            transform.translation.x = seconds.sin();
            transform.translation.y = seconds.cos();
        }
        for mut transform in world
            .query_filtered::<&mut Transform, With<RaytracedAudioListener3d>>()
            .iter_mut(world)
        {
            transform.translation.x = seconds.sin();
            transform.translation.z = seconds.cos();
        }
        for mut transform in world
            .query_filtered::<&mut Transform, With<RaytracedAudioSurface2d>>()
            .iter_mut(world)
        {
            transform.rotation = Quat::from_rotation_z(0.2 * seconds.sin());
        }
        for mut transform in world
            .query_filtered::<&mut Transform, With<RaytracedAudioSurface3d>>()
            .iter_mut(world)
        {
            transform.rotation = Quat::from_rotation_y(0.2 * seconds.sin());
        }
    }
}

/// Collects primary invalidation causes and the independent Bevy scheduling counters.
fn statistics(world: &World) -> [u64; 12] {
    let (c, s) = world
        .get_resource::<RaytracedAudioListenerTrace2d>()
        .map(|trace| {
            (
                trace.trace().map_or_else(
                    TraceCacheStatistics::default,
                    ListenerTrace2d::cache_statistics,
                ),
                trace.scheduling_statistics(),
            )
        })
        .or_else(|| {
            world
                .get_resource::<RaytracedAudioListenerTrace3d>()
                .map(|trace| {
                    (
                        trace.trace().map_or_else(
                            TraceCacheStatistics::default,
                            ListenerTrace3d::cache_statistics,
                        ),
                        trace.scheduling_statistics(),
                    )
                })
        })
        .unwrap_or_default();
    [
        c.hits,
        c.computations,
        c.forced,
        c.cold,
        c.scene_changes,
        c.listener_changes,
        c.settings_changes,
        c.source_changes,
        s.scheduled,
        s.unchanged,
        s.throttled,
        s.scene_refreshes,
    ]
}

/// Writes raw samples before terminating native runs or publishing a browser report.
fn end_frame(world: &mut World) {
    let sink_count = world
        .query_filtered::<Entity, With<AudioSink>>()
        .iter(world)
        .count();
    let diagnostics = statistics(world);
    let rendered = world
        .resource::<RenderCount>()
        .frames
        .load(Ordering::Relaxed);
    let mut state = world.resource_mut::<FrameBenchmark>();
    if state.finished || state.baseline.is_none() {
        return;
    }
    state.sink_bounds[0] = state.sink_bounds[0].min(sink_count);
    state.sink_bounds[1] = state.sink_bounds[1].max(sink_count);
    let update = state.frame_start.elapsed().as_secs_f64() * 1000.0;
    state.update_ms.push(update);
    if state.started.elapsed().as_secs_f64() < state.warmup + state.duration {
        return;
    }
    state.finished = true;
    let baseline = state.baseline.expect("measurement began");
    let delta: Vec<_> = diagnostics
        .iter()
        .zip(baseline)
        .map(|(end, start)| end.saturating_sub(start))
        .collect();
    let elapsed: f64 = state.frame_ms.iter().sum();
    let frames = u32::try_from(state.frame_ms.len()).expect("bounded frame sample count");
    let mut report = json!({"schema":1, "application":state.name, "configuration":state.config,
        "fps":f64::from(frames)*1000.0/elapsed, "frame_ms":state.frame_ms, "main_update_ms":state.update_ms,
        "render_schedules":rendered.saturating_sub(state.render_start), "spawned_including_warmup":state.churn[0],
        "despawned_including_warmup":state.churn[1], "statistics":delta,
        "statistics_fields":["hits","computations","forced","cold","scene_changes","listener_changes","settings_changes","source_changes","scheduled","unchanged","throttled","scene_refreshes"],
        "processed_sink_bounds":state.sink_bounds, "processed_sinks":state.sink_bounds[1],
        "profiled":cfg!(feature="frame-profile") && state.config["flamegraph"].is_string(),
        "present_mode":"AutoNoVsync", "scenario":"active-v1", "warmup_s":state.warmup, "duration_s":elapsed/1000.0});
    let output = state.config["output"].as_str().map(str::to_owned);
    #[cfg(all(feature = "frame-profile", target_os = "linux"))]
    if let Some(guard) = state.profiler.take() {
        guard
            .report()
            .build()
            .expect("sampling report")
            .flamegraph(
                std::fs::File::create(state.config["flamegraph"].as_str().expect("profile path"))
                    .expect("profile file"),
            )
            .expect("flamegraph");
    }
    if let Ok(window) = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
    {
        report["resolution"] = json!([window.physical_width(), window.physical_height()]);
    }
    report["emitters_2d"] = json!(
        world
            .query_filtered::<Entity, With<RaytracedAudioEmitter2d>>()
            .iter(world)
            .count()
    );
    report["emitters_3d"] = json!(
        world
            .query_filtered::<Entity, With<RaytracedAudioEmitter3d>>()
            .iter(world)
            .count()
    );
    report["processed_sinks_at_finish"] = json!(
        world
            .query_filtered::<Entity, With<AudioSink>>()
            .iter(world)
            .count()
    );
    let settings = world
        .get_resource::<RaytracedAudioTracing2d>()
        .map(|t| t.settings)
        .or_else(|| {
            world
                .get_resource::<RaytracedAudioTracing3d>()
                .map(|t| t.settings)
        })
        .expect("tracing configuration");
    report["quality"] = json!({"ray_count":settings.ray_count(), "max_bounces":settings.max_bounces(),
        "escape_distance_m":settings.escape_distance_m(), "scattering":settings.scattering(),
        "permeation_ray_count":settings.permeation_ray_count(), "seed":settings.seed(), "record_rays":settings.records_rays()});
    report["cache_enabled"] = json!(settings.reuses_results());
    report["platform"] = json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH});
    report["gpu"] = json!(world.resource::<RenderCount>().gpu.get());
    let report = serde_json::to_string(&report).expect("serializable frame report");
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(output) = output {
            std::fs::write(output, &report).expect("frame report file");
        }
        println!("ACOUSTIC_FRAME_REPORT {report}");
        world.write_message(AppExit::Success);
    }
    #[cfg(target_arch = "wasm32")]
    {
        drop(output);
        frame_result(&report);
    }
}
