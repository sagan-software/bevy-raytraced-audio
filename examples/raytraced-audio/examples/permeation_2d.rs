//! Wall thickness: music behind a thin wooden wall stays audible, behind thick stone it almost vanishes.
//!
//! You (the blue circle) stand outside two closed rooms. The left one is a wooden shed with one
//! thin wall; the right one is a stone vault with three thick layers. A music speaker (the red
//! circle) moves between the rooms every few seconds. No ray can find a way in, so all you hear is
//! what passes straight through the walls. The orange lines show those paths: one straight from
//! you, and more from the spots where your rays first hit a wall. Each wall face they cross keeps
//! part of the sound, and it keeps far more bass than treble.
//!
//! Try this: listen as the speaker switches rooms (or press Tab). In the shed the music is dull
//! but clear; in the vault only a faint thump of bass is left. Watch the bars under "Leaks
//! through the walls" fall at the same time.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale},
    camera::ScalingMode,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::{AcousticMaterial, BandAbsorption, BandGain, RayKind, RayTraceSettings};
use bevy_raytraced_audio_2d::{
    RayKindMask, RaytracedAudio2dDebugPlugin, RaytracedAudio2dPlugin, RaytracedAudioDebugDraw2d,
    RaytracedAudioEmitter2d, RaytracedAudioListener2d, RaytracedAudioPlayer,
    RaytracedAudioRayResponse2d, RaytracedAudioSurface2d,
};

/// Half the inside size of each room in meters.
const ROOM_HALF: Vec2 = Vec2::new(2.2, 1.8);
/// Centres of the wooden shed (left) and the stone vault (right).
const ROOMS: [Vec2; 2] = [Vec2::new(-3.5, 1.5), Vec2::new(3.5, 1.5)];
/// Seconds between automatic speaker moves.
const SWITCH_EVERY_S: f32 = 8.0;
/// Static part of the on-screen explanation.
const INTRO: &str = "WALL THICKNESS\n\n\
You (blue) stand outside two closed\n\
rooms. Music (red) plays inside one.\n\
Left: wooden shed, one thin wall.\n\
Right: stone vault, three thick layers.\n\n\
Orange: sound passing through walls.\n\
Every wall face keeps some of it, and\n\
keeps more bass than treble. Thin wood\n\
lets the music through dull; the vault\n\
leaves only a faint thump of bass.\n\n\
Tab  move the speaker now";

/// Starts the example.
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
                        title: "Bevy ray-traced audio: wall thickness".to_owned(),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#bevy-canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AudioPlugin {
                    default_spatial_scale: SpatialScale::new(0.25),
                    ..default()
                }),
        )
        .add_plugins((
            RaytracedAudio2dPlugin::default().with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(192)
                    .expect("ray count is positive")
                    .with_max_bounces(6)
                    // One in four rays also casts an orange ray from its first wall hit.
                    .with_permeation_ray_count(48),
            ),
            RaytracedAudio2dDebugPlugin,
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .insert_resource(RaytracedAudioDebugDraw2d {
            kinds: RayKindMask::NONE.with(RayKind::Permeation),
            ray_speed_m_per_s: 15.0,
            ..default()
        })
        .insert_resource(SpeakerRoom {
            index: 0,
            timer: Timer::from_seconds(SWITCH_EVERY_S, TimerMode::Repeating),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (move_speaker, update_hud))
        .run();
}

/// Which room the speaker is in, and when it moves next.
#[derive(Resource)]
struct SpeakerRoom {
    /// Index into [`ROOMS`].
    index: usize,
    /// Time until the speaker moves on its own.
    timer: Timer,
}

/// Marks the looping music speaker.
#[derive(Component)]
struct Speaker;

/// Marks the text panel.
#[derive(Component)]
struct Hud;

/// Thin planks: each face lets most of the bass and some treble through.
fn wood() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.08, 0.08).expect("bounded absorption"))
        .try_with_transmission(BandGain::try_new(0.8, 0.6, 0.35).expect("bounded transmission"))
        .expect("wood stays within its energy budget")
}

/// Heavy stone: each face lets a little bass and almost no treble through.
fn stone() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.02, 0.03, 0.05).expect("bounded absorption"))
        .try_with_transmission(BandGain::try_new(0.65, 0.4, 0.15).expect("bounded transmission"))
        .expect("stone stays within its energy budget")
}

/// Spawns a rectangular wall with a sprite and four acoustic faces.
fn spawn_wall(
    commands: &mut Commands<'_, '_>,
    center: Vec2,
    size: Vec2,
    color: Color,
    material: AcousticMaterial,
) {
    let half = size / 2.0;
    let corners = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        half,
        Vec2::new(-half.x, half.y),
    ];
    commands
        .spawn((
            Sprite::from_color(color, size),
            Transform::from_translation(center.extend(0.0)),
        ))
        .with_children(|wall| {
            for (start, end) in corners.iter().zip(corners.iter().cycle().skip(1)) {
                wall.spawn((
                    RaytracedAudioSurface2d::new(*start, *end, material)
                        .expect("wall faces have distinct finite endpoints"),
                    Transform::default(),
                ));
            }
        });
}

/// Spawns a closed ring of four walls around a room's inside area.
fn spawn_ring(
    commands: &mut Commands<'_, '_>,
    center: Vec2,
    inner_half: Vec2,
    thickness: f32,
    color: Color,
    material: AcousticMaterial,
) {
    let across = Vec2::new((inner_half.x + thickness) * 2.0, thickness);
    let up = Vec2::new(thickness, inner_half.y * 2.0);
    let offset_y = inner_half.y + thickness / 2.0;
    let offset_x = inner_half.x + thickness / 2.0;
    for (position, size) in [
        (Vec2::new(0.0, offset_y), across),
        (Vec2::new(0.0, -offset_y), across),
        (Vec2::new(offset_x, 0.0), up),
        (Vec2::new(-offset_x, 0.0), up),
    ] {
        spawn_wall(commands, center + position, size, color, material);
    }
}

/// Builds both rooms, the listener, the speaker, the camera, and the text panel.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            // Always shows both rooms plus room on the left for the text panel.
            scaling_mode: ScalingMode::AutoMin {
                min_width: 24.0,
                min_height: 9.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(-4.5, 0.0, 0.0),
    ));

    let [shed, vault] = ROOMS;
    // The shed: one thin wooden wall, so sound crosses two faces.
    spawn_ring(
        &mut commands,
        shed,
        ROOM_HALF,
        0.1,
        Color::srgb(0.62, 0.42, 0.22),
        wood(),
    );
    // The vault: three stone layers with air gaps, so sound crosses six faces.
    for layer in 0_u8..3 {
        let inner = ROOM_HALF + Vec2::splat(f32::from(layer) * 0.25);
        spawn_ring(
            &mut commands,
            vault,
            inner,
            0.15,
            Color::srgb(0.5, 0.5, 0.48),
            stone(),
        );
    }
    for (center, label) in [(shed, "wooden shed"), (vault, "stone vault")] {
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: FontSize::Px(48.0),
                ..default()
            },
            // Text2d is laid out in pixels; scaling it down makes it about half a meter tall.
            Transform::from_translation((center + Vec2::new(0.0, ROOM_HALF.y + 1.2)).extend(1.0))
                .with_scale(Vec3::splat(0.012)),
        ));
    }

    commands.spawn((
        RaytracedAudioListener2d,
        SpatialListener::new(0.5),
        Mesh2d(meshes.add(Circle::new(0.25))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(0.0, -3.0, 1.0),
    ));

    commands.spawn((
        Speaker,
        RaytracedAudioEmitter2d,
        RaytracedAudioPlayer::new(asset_server.load("audio/music_loop.ogg")),
        PlaybackSettings::LOOP.with_spatial(true),
        Mesh2d(meshes.add(Circle::new(0.35))),
        MeshMaterial2d(materials.add(Color::srgb(0.95, 0.3, 0.2))),
        Transform::from_translation(shed.extend(1.0)),
    ));

    commands.spawn((
        Hud,
        Text::new(INTRO),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.85)),
    ));
}

/// Moves the speaker to the other room every few seconds, or right away with Tab.
fn move_speaker(
    keys: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time>,
    mut room: ResMut<'_, SpeakerRoom>,
    mut speaker: Single<'_, '_, &mut Transform, With<Speaker>>,
) {
    let pressed = keys.just_pressed(KeyCode::Tab);
    if pressed {
        room.timer.reset();
    }
    if pressed || room.timer.tick(time.delta()).just_finished() {
        room.index = (room.index + 1) % ROOMS.len();
    }
    let target = ROOMS.get(room.index).copied().unwrap_or_default();
    speaker.translation = target.extend(1.0);
}

/// Draws a ten-step text meter for a value in `[0, 1]`.
fn meter(value: f32) -> String {
    let filled = value.clamp(0.0, 1.0) * 10.0;
    let bar: String = (0_u8..10)
        .map(|step| {
            if f32::from(step) + 0.5 < filled {
                '#'
            } else {
                '.'
            }
        })
        .collect();
    format!("[{bar}]")
}

/// Converts an amplitude gain into decibels, floored at -60 dB.
fn decibels(gain: f32) -> f32 {
    20.0 * gain.max(0.001).log10()
}

/// Writes the speaker's live permeation per band under the explanation.
fn update_hud(
    room: Res<'_, SpeakerRoom>,
    speaker: Single<'_, '_, Option<&RaytracedAudioRayResponse2d>, With<Speaker>>,
    mut hud: Single<'_, '_, &mut Text, With<Hud>>,
) {
    let Some(traced) = *speaker else {
        return;
    };
    let response = traced.response();
    let permeation = response.permeation();
    let filter = response.filter();
    let room_name = if room.index == 0 {
        "wooden shed (2 faces)"
    } else {
        "stone vault (6 faces)"
    };
    hud.0 = format!(
        "{INTRO}\n\n\
         Speaker in   {room_name}\n\
         Moves in     {:.0} s\n\n\
         Leaks through the walls\n\
         Bass         {} {:.2}\n\
         Middle       {} {:.2}\n\
         Treble       {} {:.3}\n\n\
         Filter       bass {:.2}  treble {:.3}\n\
         Bass level   {:.0} dB",
        room.timer.remaining_secs().ceil(),
        meter(permeation.low()),
        permeation.low(),
        meter(permeation.mid()),
        permeation.mid(),
        meter(permeation.high()),
        permeation.high(),
        filter.gain_lf(),
        filter.gain_hf(),
        decibels(filter.gain_lf()),
    );
}
