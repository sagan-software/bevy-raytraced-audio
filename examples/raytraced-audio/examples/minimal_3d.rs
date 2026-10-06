//! Runs a spatial Bevy sound through a visible 3D acoustic wall.

use bevy::{
    diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::AcousticMaterial;
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioResponse3d, RaytracedAudioSurface3d,
};

/// Vertical height of the example's two-triangle wall.
const WALL_HEIGHT_METERS: f32 = 2.0;
/// Half the wall width along the Z axis.
const WALL_HALF_WIDTH_METERS: f32 = 1.5;

/// Starts Bevy's normal asset, render, spatial-audio, and window plugins.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy ray-traced audio: 3D".to_owned(),
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1280, 720),
                canvas: Some("#bevy-canvas".to_owned()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            RaytracedAudio3dPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_source, draw_path, update_fps))
        .run();
}

/// Adds a movable sound, one listener, two acoustic wall triangles, and a room view.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, 5.0, 9.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 1_400_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 7.0, 5.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 8.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.2, 0.24))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(
            0.12,
            WALL_HEIGHT_METERS,
            WALL_HALF_WIDTH_METERS * 2.0,
        ))),
        MeshMaterial3d(materials.add(Color::srgb(0.95, 0.48, 0.2))),
        Transform::from_xyz(0.0, WALL_HEIGHT_METERS / 2.0, 0.0),
    ));

    // The acoustic square is split into two triangles that share one world transform.
    let first_triangle = [
        Vec3::new(0.0, 0.0, -WALL_HALF_WIDTH_METERS),
        Vec3::new(0.0, WALL_HEIGHT_METERS, -WALL_HALF_WIDTH_METERS),
        Vec3::new(0.0, WALL_HEIGHT_METERS, WALL_HALF_WIDTH_METERS),
    ];
    let second_triangle = [
        Vec3::new(0.0, 0.0, -WALL_HALF_WIDTH_METERS),
        Vec3::new(0.0, WALL_HEIGHT_METERS, WALL_HALF_WIDTH_METERS),
        Vec3::new(0.0, 0.0, WALL_HALF_WIDTH_METERS),
    ];
    for vertices in [first_triangle, second_triangle] {
        commands.spawn((
            RaytracedAudioSurface3d::new(vertices, AcousticMaterial::default())
                .expect("the example wall triangle has nonzero area"),
            Transform::default(),
        ));
    }

    commands.spawn((
        Name::new("Acoustic listener"),
        RaytracedAudioListener3d,
        SpatialListener::new(0.2),
        Mesh3d(meshes.add(Sphere::new(0.16).mesh().uv(16, 12))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.95, 0.55))),
        Transform::from_xyz(-3.0, 1.0, 0.0),
    ));

    commands.spawn((
        Name::new("Looping sound emitter"),
        RaytracedAudioEmitter3d,
        AudioPlayer::new(asset_server.load("audio/bevy-raytraced-audio-chime.wav")),
        PlaybackSettings::LOOP.with_spatial(true),
        Mesh3d(meshes.add(Sphere::new(0.2).mesh().uv(24, 16))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(3.0, 1.0, 0.0),
        MovingEmitter,
    ));

    commands.spawn((
        Text::new("The source passes the finite wall edge; the path color follows the response."),
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

/// Marks the single source whose path and position are shown in the example.
#[derive(Component)]
struct MovingEmitter;

/// Marks the text node that displays the measured frame rate.
#[derive(Component)]
struct FpsReadout;

/// Moves the emitter across the triangle wall's finite Z extent.
fn animate_source(
    time: Res<'_, Time>,
    mut emitter: Single<'_, '_, &mut Transform, With<MovingEmitter>>,
) {
    emitter.translation.z = (time.elapsed_secs() * 0.7).sin() * 3.0;
}

/// Draws the current direct path in green or red from the response component.
fn draw_path(
    mut gizmos: Gizmos<'_, '_>,
    listener: Single<'_, '_, &GlobalTransform, With<RaytracedAudioListener3d>>,
    emitter: Single<'_, '_, (&GlobalTransform, &RaytracedAudioResponse3d), With<MovingEmitter>>,
) {
    let color = if emitter.1.response().direct.is_occluded() {
        Color::srgb(1.0, 0.15, 0.12)
    } else {
        Color::srgb(0.1, 1.0, 0.45)
    };
    gizmos.line(listener.translation(), emitter.0.translation(), color);
}

/// Displays the frame-rate diagnostic so the stress target is visible during review.
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
