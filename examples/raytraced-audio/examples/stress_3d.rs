//! Exercises the 3D adapter with 16 moving sources and 32 explicit triangles.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale},
    diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::AcousticMaterial;
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioSurface3d,
};
use std::f32::consts::TAU;

/// Number of traced sources in this fixed reproducible workload.
const EMITTER_COUNT: usize = 16;
/// Exact `f32` representation of the fixed emitter count.
const EMITTER_COUNT_FLOAT: f32 = 16.0;
/// Number of acoustic surfaces rebuilt and queried each frame.
const SURFACE_COUNT: usize = 32;

/// Starts the stress scene with one spatial audio voice and 16 traced emitters.
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // The bundled WAV uses Bevy's default metadata and needs no sidecar request.
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy ray-traced audio: 3D stress".to_owned(),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#bevy-canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AudioPlugin {
                    default_spatial_scale: SpatialScale::new(1.0),
                    ..default()
                }),
        )
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            RaytracedAudio3dPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_emitters, update_fps))
        .run();
}

/// Creates a listener, moving source ring, camera, floor, and triangle field.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        // Small low-detail stress markers gain little from multisampling.
        Msaa::Off,
        Transform::from_xyz(12.0, 10.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.2, 0.24),
            unlit: true,
            ..default()
        })),
    ));
    commands.spawn((
        RaytracedAudioListener3d,
        Mesh3d(meshes.add(Sphere::new(0.22).mesh().uv(12, 8))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.95, 0.55),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.3, 0.0),
    ));

    let emitter_mesh = meshes.add(Sphere::new(0.12).mesh().uv(8, 6));
    let emitter_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.6, 1.0),
        unlit: true,
        ..default()
    });
    for emitter_index in 0..EMITTER_COUNT {
        let phase = TAU * (small_index(emitter_index) / EMITTER_COUNT_FLOAT);
        let mut emitter = commands.spawn((
            RaytracedAudioEmitter3d,
            Mesh3d(emitter_mesh.clone()),
            MeshMaterial3d(emitter_material.clone()),
            Transform::from_xyz(5.0 * phase.cos(), 0.3, 5.0 * phase.sin()),
            OrbitPhase { radians: phase },
        ));
        if emitter_index == 0 {
            emitter.insert((
                AudioPlayer::new(asset_server.load("audio/bevy-raytraced-audio-chime.wav")),
                PlaybackSettings::LOOP.with_spatial(true),
            ));
        }
    }

    // Each trace triangle has a small visual marker with shared mesh and material assets.
    let wall_mesh = meshes.add(Cuboid::new(0.025, 0.24, 0.24));
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.48, 0.2),
        unlit: true,
        ..default()
    });
    for surface_index in 0..SURFACE_COUNT {
        let column = surface_index % 32;
        let row = surface_index / 32;
        let x = (small_index(column) - 15.5) * 0.4;
        let z = (small_index(row) - 3.5) * 0.4;
        let vertices = [
            Vec3::new(0.0, -0.12, -0.12),
            Vec3::new(0.0, 0.12, -0.12),
            Vec3::new(0.0, 0.12, 0.12),
        ];
        commands.spawn((
            RaytracedAudioSurface3d::new(vertices, AcousticMaterial::default())
                .expect("stress triangles have nonzero area"),
            Mesh3d(wall_mesh.clone()),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_xyz(x, 1.0, z),
        ));
    }

    commands.spawn((
        Text::new("16 emitters"),
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

/// Moves all 16 sources together while preserving their fixed spacing.
fn animate_emitters(
    time: Res<'_, Time>,
    mut emitters: Query<'_, '_, (&mut Transform, &OrbitPhase)>,
) {
    for (mut transform, phase) in &mut emitters {
        let angle = time.elapsed_secs().mul_add(0.15, phase.radians);
        transform.translation.x = 5.0 * angle.cos();
        transform.translation.z = 5.0 * angle.sin();
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
