//! Runs a spatial Bevy sound through a visible, movable 2D acoustic scene.

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
    RaytracedAudioReflectionPaths2d, RaytracedAudioResponse2d, RaytracedAudioSurface2d,
};

/// Keeps enough room around the wall for the animated source to pass its endpoint.
const VIEW_HEIGHT_METERS: f32 = 7.0;
/// Limits the wall segment to a visible doorway-sized barrier.
const WALL_HALF_HEIGHT_METERS: f32 = 1.25;

/// Starts the Bevy 2D example with the built-in audio plugin still enabled.
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy ray-traced audio: 2D".to_owned(),
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
        .add_systems(
            Update,
            (animate_source, move_listener, draw_path, update_fps),
        )
        .run();
}

/// Adds a camera, a looping spatial source, one listener, and one explicit wall.
fn setup(mut commands: Commands<'_, '_>, asset_server: Res<'_, AssetServer>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIEW_HEIGHT_METERS,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    commands.spawn((
        Name::new("Acoustic listener"),
        RaytracedAudioListener2d,
        SpatialListener::new(0.2),
        Sprite::from_color(Color::srgb(0.2, 0.95, 0.55), Vec2::splat(0.32)),
        Transform::from_xyz(-3.5, 0.0, 1.0),
    ));

    commands.spawn((
        Name::new("Looping sound emitter"),
        RaytracedAudioEmitter2d,
        RaytracedAudioReflectionPaths2d::default(),
        AudioPlayer::new(asset_server.load("audio/bevy-raytraced-audio-chime.wav")),
        PlaybackSettings::LOOP.with_spatial(true),
        Sprite::from_color(Color::srgb(0.25, 0.6, 1.0), Vec2::splat(0.3)),
        Transform::from_xyz(-2.0, 0.0, 1.0),
        MovingEmitter,
    ));

    commands.spawn((
        Name::new("Acoustic wall"),
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -WALL_HALF_HEIGHT_METERS),
            Vec2::new(0.0, WALL_HALF_HEIGHT_METERS),
            AcousticMaterial::default(),
        )
        .expect("the example wall has distinct finite endpoints"),
        Sprite::from_color(
            Color::srgb(0.95, 0.48, 0.2),
            Vec2::new(0.12, WALL_HALF_HEIGHT_METERS * 2.0),
        ),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Text::new("Arrow keys move the listener. Green/red: direct path. Cyan: reflections."),
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

/// Moves the emitter above and below the finite wall endpoint.
fn animate_source(
    time: Res<'_, Time>,
    mut emitter: Single<'_, '_, &mut Transform, With<MovingEmitter>>,
) {
    emitter.translation.y = (time.elapsed_secs() * 0.7).sin() * 3.0;
}

/// Moves the listener with the arrow keys while keeping it in the XY plane.
fn move_listener(
    keys: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time>,
    mut listener: Single<'_, '_, &mut Transform, With<RaytracedAudioListener2d>>,
) {
    let mut direction = Vec2::ZERO;
    direction.x += f32::from(keys.pressed(KeyCode::ArrowRight));
    direction.x -= f32::from(keys.pressed(KeyCode::ArrowLeft));
    direction.y += f32::from(keys.pressed(KeyCode::ArrowUp));
    direction.y -= f32::from(keys.pressed(KeyCode::ArrowDown));
    listener.translation += direction.extend(0.0) * (time.delta_secs() * 1.5);
}

/// Draws the direct path and each first-order reflection polyline.
fn draw_path(
    mut gizmos: Gizmos<'_, '_>,
    listener: Single<'_, '_, &GlobalTransform, With<RaytracedAudioListener2d>>,
    emitter: Single<
        '_,
        '_,
        (
            &GlobalTransform,
            &RaytracedAudioResponse2d,
            &RaytracedAudioReflectionPaths2d,
        ),
        With<MovingEmitter>,
    >,
) {
    let color = if emitter.1.response().direct.is_occluded() {
        Color::srgb(1.0, 0.15, 0.12)
    } else {
        Color::srgb(0.1, 1.0, 0.45)
    };
    gizmos.line_2d(
        listener.translation().truncate(),
        emitter.0.translation().truncate(),
        color,
    );
    let reflection_color = Color::srgb(0.1, 0.8, 1.0);
    for path in emitter.2.paths() {
        let reflection_point = render_point_2d(path.reflection_point());
        let source_point = emitter.0.translation().truncate();
        let listener_point = listener.translation().truncate();
        gizmos.line_2d(source_point, reflection_point, reflection_color);
        gizmos.line_2d(reflection_point, listener_point, reflection_color);
    }
}

/// Converts solver meter coordinates to Bevy's single-precision render coordinates.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Bevy Gizmos use f32 coordinates while the acoustic solver stores f64 meters."
)]
const fn render_point_2d(point: bevy_raytraced_audio::SolverPoint2d) -> Vec2 {
    Vec2::new(point.x_m() as f32, point.y_m() as f32)
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
