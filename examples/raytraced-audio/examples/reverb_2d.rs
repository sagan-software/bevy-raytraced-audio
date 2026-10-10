//! Echo and room size: the same clap sounds different in a bathroom, a hall, and a cathedral.
//!
//! You (the blue circle) clap in an empty room. White lines are sound rays leaving you and
//! bouncing off the walls, slowed down so you can watch them. Whenever a bounce point has a clear
//! line back to you, a blue line marks an echo. The trace turns those rays into a reverb: how far
//! rays travel between bounces sets the room size, which sets how long the echo rings (RT60) and
//! how late the first reflection arrives (pre-delay).
//!
//! Try this: press 1, 2, and 3 to switch rooms (each switch claps once), then press C to fill the
//! room with clutter. The boxes block the blue echo lines and soak up sound, so a cluttered room
//! sounds much drier, like a garage full of boxes.

use bevy::{
    asset::AssetMetaCheck,
    audio::Volume,
    camera::ScalingMode,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::{
    AcousticMaterial, BandAbsorption, RayKind, RayTraceSettings, SPEED_OF_SOUND_M_PER_S,
};
use bevy_raytraced_audio_2d::{
    RayKindMask, RaytracedAudio2dDebugPlugin, RaytracedAudio2dPlugin, RaytracedAudioDebugDraw2d,
    RaytracedAudioEmitter2d, RaytracedAudioListener2d, RaytracedAudioListenerTrace2d,
    RaytracedAudioPlayer, RaytracedAudioSurface2d,
};

/// The rooms you can switch between: name, side length in meters, and key.
const ROOMS: [(&str, f32, KeyCode); 3] = [
    ("Bathroom", 3.0, KeyCode::Digit1),
    ("Hall", 12.0, KeyCode::Digit2),
    ("Cathedral", 30.0, KeyCode::Digit3),
];
/// Static part of the on-screen explanation.
const INTRO: &str = "ECHO AND ROOM SIZE\n\n\
You (blue) clap in an empty room.\n\n\
White: rays leaving you, bouncing.\n\
Blue: a bounce point with a clear line\n\
  back to you. That is the echo.\n\
Big rooms: long bounces, so the echo\n\
  arrives later and rings longer.\n\
Clutter blocks and soaks up echoes,\n\
  so a full garage sounds dry.\n\n\
Space  clap        G  gunshot\n\
1 bathroom   2 hall   3 cathedral\n\
C      clutter on / off";

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
                        title: "Bevy ray-traced audio: echo and room size".to_owned(),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#bevy-canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            RaytracedAudio2dPlugin::default().with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(160)
                    .expect("ray count is positive")
                    .with_max_bounces(6),
            ),
            RaytracedAudio2dDebugPlugin,
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .insert_resource(RaytracedAudioDebugDraw2d {
            kinds: RayKindMask::NONE.with(RayKind::Primary).with(RayKind::Echo),
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_keys, rebuild_room, update_hud).chain())
        .run();
}

/// The selected room and the sounds you can make in it.
#[derive(Resource)]
struct Room {
    /// Index into [`ROOMS`].
    index: usize,
    /// Whether the room is filled with absorbing boxes.
    clutter: bool,
    /// Whether the walls need to be rebuilt.
    dirty: bool,
    /// Hand clap recording.
    clap: Handle<AudioSource>,
    /// Gunshot recording.
    gunshot: Handle<AudioSource>,
}

impl Room {
    /// Returns the side length of the selected room in meters.
    fn size(&self) -> f32 {
        ROOMS.get(self.index).map_or(1.0, |room| room.1)
    }
}

/// Marks walls and clutter that are rebuilt when the room changes.
#[derive(Component)]
struct RoomPart;

/// Marks the text panel.
#[derive(Component)]
struct Hud;

/// Returns where the listener stands in a room of the given size.
///
/// Standing a little off-centre keeps the echoes from all lining up.
fn listener_spot(size: f32) -> Vec2 {
    Vec2::new(-0.15, -0.1) * size
}

/// Spawns the camera, listener, text panel, and the first room.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        RaytracedAudioListener2d,
        // The circle is scaled with the room size so it stays visible at every zoom.
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(0.0, 0.0, 1.0),
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
    commands.insert_resource(Room {
        index: 1,
        clutter: false,
        dirty: true,
        clap: asset_server.load("audio/clap.ogg"),
        gunshot: asset_server.load("audio/gunshot_1.ogg"),
    });
}

/// Hard painted walls: a little absorption, so big rooms ring for seconds.
fn plaster() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.08, 0.1, 0.14).expect("bounded absorption"))
}

/// Boxes, sofas, and coats: they absorb most of the sound that hits them.
fn clutter() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.45, 0.6, 0.7).expect("bounded absorption"))
}

/// Spawns a rotated rectangle with a sprite and four acoustic faces.
fn spawn_box(
    commands: &mut Commands<'_, '_>,
    transform: Transform,
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
        .spawn((RoomPart, Sprite::from_color(color, size), transform))
        .with_children(|part| {
            for (start, end) in corners.iter().zip(corners.iter().cycle().skip(1)) {
                part.spawn((
                    RaytracedAudioSurface2d::new(*start, *end, material)
                        .expect("box faces have distinct finite endpoints"),
                    Transform::default(),
                ));
            }
        });
}

/// Plays a recording at the listener; the adapter adds the traced room reverb to it.
fn play(commands: &mut Commands<'_, '_>, sound: Handle<AudioSource>, at: Vec2) {
    commands.spawn((
        RaytracedAudioEmitter2d,
        RaytracedAudioPlayer::new(sound),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.8)),
        Transform::from_translation(at.extend(1.0)),
    ));
}

/// Claps, fires, switches rooms, and toggles clutter.
fn handle_keys(
    mut commands: Commands<'_, '_>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut room: ResMut<'_, Room>,
) {
    // Clapping after every change makes the before/after easy to compare.
    let mut clap = keys.just_pressed(KeyCode::Space);
    for (index, (_, _, key)) in ROOMS.iter().enumerate() {
        if keys.just_pressed(*key) && room.index != index {
            room.index = index;
            room.dirty = true;
            clap = true;
        }
    }
    if keys.just_pressed(KeyCode::KeyC) {
        room.clutter = !room.clutter;
        room.dirty = true;
        clap = true;
    }
    let spot = listener_spot(room.size());
    if clap {
        play(&mut commands, room.clap.clone(), spot);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        play(&mut commands, room.gunshot.clone(), spot);
    }
}

/// Rebuilds the walls and clutter, and frames the camera around the room.
fn rebuild_room(
    mut commands: Commands<'_, '_>,
    mut room: ResMut<'_, Room>,
    parts: Query<'_, '_, Entity, With<RoomPart>>,
    mut listener: Single<'_, '_, &mut Transform, With<RaytracedAudioListener2d>>,
    camera: Single<'_, '_, (&mut Projection, &mut Transform), Without<RaytracedAudioListener2d>>,
    mut draw: ResMut<'_, RaytracedAudioDebugDraw2d>,
) {
    if !room.dirty {
        return;
    }
    room.dirty = false;
    for part in &parts {
        commands.entity(part).despawn();
    }
    let size = room.size();
    let half = size / 2.0;
    let wall = (size * 0.03).max(0.1);
    let color = Color::srgb(0.75, 0.74, 0.7);
    for (center, extent) in [
        (Vec2::new(0.0, half), Vec2::new(size + wall, wall)),
        (Vec2::new(0.0, -half), Vec2::new(size + wall, wall)),
        (Vec2::new(half, 0.0), Vec2::new(wall, size + wall)),
        (Vec2::new(-half, 0.0), Vec2::new(wall, size + wall)),
    ] {
        spawn_box(
            &mut commands,
            Transform::from_translation(center.extend(0.0)),
            extent,
            color,
            plaster(),
        );
    }

    let spot = listener_spot(size);
    listener.translation = spot.extend(1.0);
    listener.scale = Vec3::splat(size * 0.025);

    if room.clutter {
        let color = Color::srgb(0.62, 0.45, 0.28);
        for column in -2_i8..=2 {
            for row in -2_i8..=2 {
                let center = Vec2::new(f32::from(column), f32::from(row)) * size * 0.18
                    + Vec2::new(0.04, 0.06) * size;
                if center.distance(spot) < size * 0.12 {
                    continue;
                }
                let transform = Transform::from_translation(center.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(f32::from(column + row) * 0.45));
                spawn_box(
                    &mut commands,
                    transform,
                    Vec2::splat(size * 0.08),
                    color,
                    clutter(),
                );
            }
        }
    }

    // Leaves the left part of the window free for the text panel.
    let (mut projection, mut camera_transform) = camera.into_inner();
    *projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: size * 1.75,
            min_height: size * 1.25,
        },
        ..OrthographicProjection::default_2d()
    });
    camera_transform.translation.x = -size * 0.28;
    // Rays travel two room lengths per second, so every room takes as long to watch.
    draw.ray_speed_m_per_s = size * 2.0;
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

/// Writes the listener's live reverb estimate under the explanation.
fn update_hud(
    room: Res<'_, Room>,
    trace: Res<'_, RaytracedAudioListenerTrace2d>,
    draw: Res<'_, RaytracedAudioDebugDraw2d>,
    mut hud: Single<'_, '_, &mut Text, With<Hud>>,
) {
    let Some(trace) = trace.trace() else {
        return;
    };
    let reverb = trace.reverb();
    let name = ROOMS.get(room.index).map_or("", |room| room.0);
    let size = room.size();
    let slowdown = SPEED_OF_SOUND_M_PER_S / f64::from(draw.ray_speed_m_per_s);
    hud.0 = format!(
        "{INTRO}\n\n\
         Room         {name} {size:.0} x {size:.0} m{}\n\
         Echo returns {} {:>3.0}%\n\
         Room size    {:.1} m between bounces\n\
         Decay (RT60) {:.2} s\n\
         Pre-delay    {:.0} ms\n\
         Reverb send  {} {:.2}\n\n\
         Rays drawn {slowdown:.0}x slower than sound",
        if room.clutter { ", cluttered" } else { "" },
        meter(reverb.return_fraction()),
        reverb.return_fraction() * 100.0,
        reverb.mean_free_path_m(),
        reverb.decay_time_s(),
        reverb.reflections_delay_s() * 1000.0,
        meter(reverb.wet_gain()),
        reverb.wet_gain(),
    );
}
