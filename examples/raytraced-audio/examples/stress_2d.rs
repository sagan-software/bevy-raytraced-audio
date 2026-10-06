//! Exercises the 2D adapter with 128 moving sources and 256 explicit wall segments.

use bevy::{
    audio::{AudioPlugin, SpatialScale},
    camera::ScalingMode,
    diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::AcousticMaterial;
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
    RaytracedAudioSurface2d,
};
use std::f32::consts::TAU;

/// Number of traced sources in this fixed reproducible workload.
const EMITTER_COUNT: usize = 128;
/// Exact `f32` representation of the fixed emitter count.
const EMITTER_COUNT_FLOAT: f32 = 128.0;
/// Number of acoustic surfaces rebuilt and queried each frame.
const SURFACE_COUNT: usize = 256;

/// Starts the stress scene with one spatial audio voice and 128 traced emitters.
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy ray-traced audio: 2D stress".to_owned(),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#bevy-canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AudioPlugin {
                    default_spatial_scale: SpatialScale::new_2d(1.0),
                    ..default()
                }),
        )
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            RaytracedAudio2dPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_emitters, update_fps))
        .run();
}

/// Creates a listener, a visible source ring, and a grid of explicit wall segments.
fn setup(mut commands: Commands<'_, '_>, asset_server: Res<'_, AssetServer>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 24.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.spawn((
        RaytracedAudioListener2d,
        Sprite::from_color(Color::srgb(0.2, 0.95, 0.55), Vec2::splat(0.34)),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    for emitter_index in 0..EMITTER_COUNT {
        let phase = TAU * (small_index(emitter_index) / EMITTER_COUNT_FLOAT);
        let mut emitter = commands.spawn((
            RaytracedAudioEmitter2d,
            Sprite::from_color(Color::srgb(0.25, 0.6, 1.0), Vec2::splat(0.12)),
            Transform::from_xyz(5.0 * phase.cos(), 5.0 * phase.sin(), 1.0),
            OrbitPhase { radians: phase },
        ));
        if emitter_index == 0 {
            emitter.insert((
                AudioPlayer::new(asset_server.load("audio/bevy-raytraced-audio-chime.wav")),
                PlaybackSettings::LOOP.with_spatial(true),
            ));
        }
    }

    // A stable grid gives every run the same geometry count and positions.
    for surface_index in 0..SURFACE_COUNT {
        let column = surface_index % 32;
        let row = surface_index / 32;
        let x = (small_index(column) - 15.5) * 0.4;
        let y = (small_index(row) - 3.5) * 0.4;
        let surface = RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -0.12),
            Vec2::new(0.0, 0.12),
            AcousticMaterial::default(),
        )
        .expect("stress segments have nonzero length");
        commands.spawn((
            surface,
            Sprite::from_color(Color::srgb(0.95, 0.48, 0.2), Vec2::new(0.035, 0.24)),
            Transform::from_xyz(x, y, 0.0),
        ));
    }

    commands.spawn((
        Text::new("128 moving emitters | 256 wall segments | one looping spatial sound"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
    commands.spawn((
        Text::new("FPS: waiting for diagnostics"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
        FpsReadout,
    ));
}

/// Stores one source's stable angle around the listener.
#[derive(Component)]
struct OrbitPhase {
    /// Starting angle in radians, used to keep sources evenly spaced.
    radians: f32,
}

/// Marks the text node that displays the measured frame rate.
#[derive(Component)]
struct FpsReadout;

/// Moves all 128 sources together while preserving their fixed spacing.
fn animate_emitters(
    time: Res<'_, Time>,
    mut emitters: Query<'_, '_, (&mut Transform, &OrbitPhase)>,
) {
    for (mut transform, phase) in &mut emitters {
        let angle = time.elapsed_secs().mul_add(0.15, phase.radians);
        transform.translation.x = 5.0 * angle.cos();
        transform.translation.y = 5.0 * angle.sin();
    }
}

/// Displays the rolling frame-rate diagnostic for the 90 FPS target.
fn update_fps(
    diagnostics: Res<'_, DiagnosticsStore>,
    mut readout: Single<'_, '_, &mut Text, With<FpsReadout>>,
) {
    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::smoothed)
    else {
        return;
    };
    readout.0 = format!("FPS: {fps:.0}");
}

/// Converts the fixed small fixture indices without losing integer precision.
fn small_index(index: usize) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(u16::MAX))
}
