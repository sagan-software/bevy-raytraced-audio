//! A walkable acoustic village built from CC0 Kenney modules.
//!
//! Rendering, collision and acoustic openings share one world plan. The listener and sources
//! use real XYZ positions: roofs, floors and basement walls remain acoustic even in cutaway view.
//! See `showcase/` for the world, controller, sound mix, interactions and regression scenarios.

#[path = "showcase/audio.rs"]
mod audio;
#[path = "showcase/controller.rs"]
mod controller;
#[path = "showcase/interface.rs"]
mod interface;
#[cfg(test)]
#[path = "showcase/tests.rs"]
mod tests;
#[path = "showcase/world.rs"]
mod world;

#[path = "frame_benchmark.rs"]
mod frame_benchmark;

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::RayTraceSettings;
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dDebugPlugin, RaytracedAudio3dPlugin, RaytracedAudioDebugDraw3d,
    RaytracedAudioTracing3d,
};

/// Starts the village with bounded ray work and a quiet default mix.
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Acoustic village • doors, floors and flowing water".to_owned(),
                        present_mode: PresentMode::AutoVsync,
                        resolution: WindowResolution::new(1280, 800),
                        canvas: Some("#bevy-canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AudioPlugin {
                    default_spatial_scale: SpatialScale::new(0.22),
                    ..default()
                }),
        )
        .add_plugins((
            RaytracedAudio3dPlugin::default(),
            RaytracedAudio3dDebugPlugin,
        ))
        .insert_resource(RaytracedAudioTracing3d {
            enabled: true,
            interval_s: 0.08,
            settings: RayTraceSettings::default()
                .try_with_ray_count(96)
                .expect("positive rays")
                .with_max_bounces(4)
                .try_with_escape_distance(45.0)
                .expect("positive distance"),
        })
        .insert_resource(RaytracedAudioDebugDraw3d {
            enabled: false,
            ray_speed_m_per_s: 18.0,
            ..default()
        })
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.57, 0.72, 0.78)))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 650.0,
            ..default()
        })
        .init_resource::<controller::View>()
        .init_resource::<audio::Mix>()
        .init_resource::<world::Village>()
        .insert_resource(world::RoomTreatment { mode: 2 })
        .add_systems(
            Startup,
            (
                world::setup,
                controller::setup,
                audio::setup,
                interface::setup,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                (
                    interface::controls,
                    controller::look,
                    controller::walk,
                    benchmark_walk,
                    controller::camera,
                    controller::head_pose,
                    world::room_treatment,
                    world::interact,
                    world::animate_gates,
                    benchmark_gates,
                    world::cutaway,
                )
                    .chain(),
                (
                    audio::runners,
                    audio::footsteps,
                    audio::probe,
                    audio::mix_sounds,
                )
                    .chain(),
                interface::hud,
                world::markers,
            ),
        )
        .add_plugins(frame_benchmark::FrameBenchmarkPlugin("showcase"))
        .run();
}

/// Drives the real player and doors along a repeatable path during frame measurements.
fn benchmark_walk(
    benchmark: Option<Res<'_, frame_benchmark::FrameBenchmark>>,
    mut player: Query<'_, '_, &mut Transform, With<controller::Player>>,
) {
    let Some(benchmark) = benchmark else {
        return;
    };
    let seconds = benchmark.seconds();
    for mut transform in &mut player {
        transform.translation.x = 3.0 * (seconds * 0.2).sin();
        transform.translation.z = 2.0f32.mul_add((seconds * 0.2).cos(), 2.0);
    }
}

/// Uses absolute time for door poses, so a slow run cannot reduce geometry churn.
fn benchmark_gates(
    benchmark: Option<Res<'_, frame_benchmark::FrameBenchmark>>,
    mut gates: Query<'_, '_, (&mut world::Gate, &mut Transform)>,
) {
    let Some(benchmark) = benchmark else {
        return;
    };
    let fraction = 0.5f32.mul_add((benchmark.seconds() * 0.5).sin(), 0.5);
    for (mut gate, mut pose) in &mut gates {
        gate.angle = fraction * gate.open_angle;
        gate.open = fraction > 0.5;
        *pose = gate
            .closed
            .with_rotation(gate.closed.rotation * Quat::from_rotation_y(gate.angle));
    }
}
