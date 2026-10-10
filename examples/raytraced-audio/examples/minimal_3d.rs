//! A 3D room with a doorway: open the door and the music is clear, close it and only bass is left.
//!
//! You (the blue ball) stand in the near room. A music speaker (the red box) plays in the far
//! room. Both rooms are built from acoustic triangles: floor, walls, and a ceiling. The ceiling and
//! the wall facing the camera are not drawn so you can look in, but they still reflect sound.
//! Every frame the listener fires sound rays in all directions; you can watch them travel and
//! bounce, slowed down to 10 m/s. White rays are bouncing, green ones found the speaker from a
//! bounce point (sound reaching you around the door frame), blue ones are echoes coming back to
//! you, and orange ones pass straight through a wall, keeping mostly the bass.
//!
//! Try this: press Space to close the door and hear the music turn dull, then walk with WASD or
//! the arrow keys and watch the green rays appear as you get a view through the doorway.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale, Volume},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::{AcousticMaterial, BandAbsorption, BandGain, RayKind, RayTraceSettings};
use bevy_raytraced_audio_3d::{
    RayKindMask, RaytracedAudio3dDebugPlugin, RaytracedAudio3dPlugin, RaytracedAudioDebugDraw3d,
    RaytracedAudioEmitter3d, RaytracedAudioListener3d, RaytracedAudioListenerTrace3d,
    RaytracedAudioPlayer, RaytracedAudioRayResponse3d, RaytracedAudioSurface3d,
};

/// Room height in meters.
const HEIGHT: f32 = 3.0;
/// Half the depth of both rooms along Z, in meters; the rooms span `x` from -6 to 6 m.
const DEPTH_HALF: f32 = 3.0;
/// Drawn wall thickness in meters; acoustically each wall is one plane through its middle.
const WALL: f32 = 0.15;
/// Half the width of the doorway in meters.
const DOOR_HALF: f32 = 0.6;
/// Height of the doorway in meters.
const DOOR_HEIGHT: f32 = 2.2;
/// Ear height of the listener and the speaker, in meters.
const EAR: f32 = 1.2;
/// Listener walking speed in meters per second.
const WALK_SPEED: f32 = 3.0;
/// Listener radius in meters, used for drawing and collisions.
const LISTENER_RADIUS: f32 = 0.3;
/// Static part of the on-screen explanation.
const INTRO: &str = "A 3D ROOM WITH A DOORWAY\n\n\
You (blue) are in the near room. Music\n\
(red) plays in the far room. Ceiling and\n\
front wall are hidden but still reflect.\n\n\
White: rays leaving you, bouncing.\n\
Green: a bounce point that can see the\n\
  speaker. More green = clearer music.\n\
Blue: echoes coming back to you.\n\
Orange: sound passing through a wall.\n\
  Walls let the bass through, not treble.\n\n\
Space         open / close the door\n\
Arrows, WASD  walk\n\
V             show / hide rays";

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
                        title: "Bevy ray-traced audio: 3D room with a doorway".to_owned(),
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
                    // Shrinks distances so the speaker in the next room stays loud enough.
                    default_spatial_scale: SpatialScale::new(0.25),
                    ..default()
                }),
        )
        .add_plugins((
            RaytracedAudio3dPlugin::default().with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(192)
                    .expect("ray count is positive")
                    .with_max_bounces(5),
            ),
            RaytracedAudio3dDebugPlugin,
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .insert_resource(RaytracedAudioDebugDraw3d {
            kinds: RayKindMask::NONE
                .with(RayKind::Primary)
                .with(RayKind::Occlusion)
                .with(RayKind::Echo)
                .with(RayKind::Permeation),
            ray_speed_m_per_s: 10.0,
            trail_m: 3.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_door, move_listener, toggle_rays, update_hud),
        )
        .run();
}

/// A wall the listener cannot walk through, with its half extents in meters.
#[derive(Component)]
struct Solid(Vec3);

/// Marks the looping music speaker.
#[derive(Component)]
struct Speaker;

/// Marks the text panel.
#[derive(Component)]
struct Hud;

/// The door entity while the door is closed, and what is needed to build it again.
#[derive(Resource)]
struct Door {
    /// The closed door's entity, or `None` while the door is open.
    entity: Option<Entity>,
    /// Door look.
    material: Handle<StandardMaterial>,
    /// Sound played when the door closes.
    slam: Handle<AudioSource>,
}

/// Plaster walls, floor, and ceiling: mostly reflective, letting some bass through.
fn plaster() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.08, 0.06).expect("bounded absorption"))
        .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("bounded transmission"))
        .expect("plaster stays within its energy budget")
}

/// Returns the four corners of the plane through the middle of a thin box.
///
/// The plane is perpendicular to the box's thinnest axis.
fn plane_corners(half: Vec3) -> [Vec3; 4] {
    if half.x <= half.y && half.x <= half.z {
        [
            Vec3::new(0.0, -half.y, -half.z),
            Vec3::new(0.0, half.y, -half.z),
            Vec3::new(0.0, half.y, half.z),
            Vec3::new(0.0, -half.y, half.z),
        ]
    } else if half.y <= half.z {
        [
            Vec3::new(-half.x, 0.0, -half.z),
            Vec3::new(half.x, 0.0, -half.z),
            Vec3::new(half.x, 0.0, half.z),
            Vec3::new(-half.x, 0.0, half.z),
        ]
    } else {
        [
            Vec3::new(-half.x, -half.y, 0.0),
            Vec3::new(half.x, -half.y, 0.0),
            Vec3::new(half.x, half.y, 0.0),
            Vec3::new(-half.x, half.y, 0.0),
        ]
    }
}

/// Spawns a thin box as two acoustic triangles, drawn only when `look` is given.
fn spawn_panel(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    center: Vec3,
    size: Vec3,
    look: Option<Handle<StandardMaterial>>,
) -> Entity {
    let [a, b, c, d] = plane_corners(size / 2.0);
    let mut panel = commands.spawn((Solid(size / 2.0), Transform::from_translation(center)));
    if let Some(material) = look {
        panel.insert((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
        ));
    }
    panel.with_children(|panel| {
        for triangle in [[a, b, c], [a, c, d]] {
            panel.spawn((
                RaytracedAudioSurface3d::new(triangle, plaster())
                    .expect("panel triangles are finite and not collinear"),
                Transform::default(),
            ));
        }
    });
    panel.id()
}

/// Builds the two rooms, the listener, the speaker, the light, the camera, and the text panel.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    let wall = materials.add(Color::srgb(0.75, 0.74, 0.7));
    let floor = materials.add(Color::srgb(0.42, 0.33, 0.25));
    let (depth, middle) = (DEPTH_HALF * 2.0, HEIGHT / 2.0);
    let side = DEPTH_HALF - DOOR_HALF;
    let panels = [
        // Floor and ceiling; the ceiling is not drawn so the camera can see in.
        (
            Vec3::new(0.0, -0.05, 0.0),
            Vec3::new(12.0, 0.1, depth),
            Some(floor),
        ),
        (
            Vec3::new(0.0, HEIGHT + 0.05, 0.0),
            Vec3::new(12.0, 0.1, depth),
            None,
        ),
        // Back, left, and right walls.
        (
            Vec3::new(0.0, middle, -DEPTH_HALF),
            Vec3::new(12.0, HEIGHT, WALL),
            Some(wall.clone()),
        ),
        (
            Vec3::new(-6.0, middle, 0.0),
            Vec3::new(WALL, HEIGHT, depth),
            Some(wall.clone()),
        ),
        (
            Vec3::new(6.0, middle, 0.0),
            Vec3::new(WALL, HEIGHT, depth),
            Some(wall.clone()),
        ),
        // The front wall faces the camera, so it is acoustic only.
        (
            Vec3::new(0.0, middle, DEPTH_HALF),
            Vec3::new(12.0, HEIGHT, WALL),
            None,
        ),
        // The shared wall around the doorway: two sides and a lintel above the door.
        (
            Vec3::new(0.0, middle, -DOOR_HALF - side / 2.0),
            Vec3::new(WALL, HEIGHT, side),
            Some(wall.clone()),
        ),
        (
            Vec3::new(0.0, middle, DOOR_HALF + side / 2.0),
            Vec3::new(WALL, HEIGHT, side),
            Some(wall.clone()),
        ),
        (
            Vec3::new(0.0, f32::midpoint(DOOR_HEIGHT, HEIGHT), 0.0),
            Vec3::new(WALL, HEIGHT - DOOR_HEIGHT, DOOR_HALF * 2.0),
            Some(wall.clone()),
        ),
    ];
    for (center, size, look) in panels {
        spawn_panel(&mut commands, &mut meshes, center, size, look);
    }
    // A low strip marks where the hidden front wall stands.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.3, WALL))),
        MeshMaterial3d(wall),
        Transform::from_xyz(0.0, 0.15, DEPTH_HALF),
    ));
    commands.insert_resource(Door {
        entity: None,
        material: materials.add(Color::srgb(0.6, 0.38, 0.2)),
        slam: asset_server.load("audio/door_close.ogg"),
    });

    commands.spawn((
        RaytracedAudioListener3d,
        SpatialListener::new(LISTENER_RADIUS * 2.0),
        Mesh3d(meshes.add(Sphere::new(LISTENER_RADIUS))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(-3.5, EAR, 1.0),
    ));
    commands.spawn((
        Speaker,
        RaytracedAudioEmitter3d,
        // The player filters the music with the traced muffling, so you hear the walls.
        RaytracedAudioPlayer::new(asset_server.load("audio/music_loop.ogg")).with_reverb_send(0.5),
        PlaybackSettings::LOOP.with_spatial(true),
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.7, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.3, 0.2),
            emissive: LinearRgba::rgb(0.8, 0.15, 0.05),
            ..default()
        })),
        Transform::from_xyz(3.5, EAR, -1.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            ..default()
        },
        Transform::from_xyz(-4.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        brightness: 400.0,
        ..default()
    });
    // A fixed three-quarter view from above; the rooms sit right of centre beside the text.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 13.0, 12.0).looking_at(Vec3::new(-2.5, 0.0, 0.5), Vec3::Y),
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

/// Opens or closes the door with Space; closing it plays a slam behind the door.
fn toggle_door(
    mut commands: Commands<'_, '_>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut door: ResMut<'_, Door>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }
    if let Some(entity) = door.entity.take() {
        commands.entity(entity).despawn();
        return;
    }
    door.entity = Some(spawn_panel(
        &mut commands,
        &mut meshes,
        Vec3::new(0.0, DOOR_HEIGHT / 2.0, 0.0),
        Vec3::new(WALL * 0.7, DOOR_HEIGHT, DOOR_HALF * 2.0),
        Some(door.material.clone()),
    ));
    commands.spawn((
        RaytracedAudioEmitter3d,
        RaytracedAudioPlayer::new(door.slam.clone()),
        PlaybackSettings::DESPAWN
            .with_spatial(true)
            .with_volume(Volume::Linear(0.8)),
        Transform::from_xyz(0.3, EAR, 0.0),
    ));
}

/// Walks the listener across the floor with the arrow keys or WASD, sliding along walls.
fn move_listener(
    keys: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time>,
    mut listener: Single<'_, '_, &mut Transform, With<RaytracedAudioListener3d>>,
    solids: Query<'_, '_, (&Transform, &Solid), Without<RaytracedAudioListener3d>>,
) {
    let pressed = |a: KeyCode, b: KeyCode| f32::from(keys.pressed(a) || keys.pressed(b));
    // Screen up is away from the camera, along -Z.
    let direction = Vec3::new(
        pressed(KeyCode::ArrowRight, KeyCode::KeyD) - pressed(KeyCode::ArrowLeft, KeyCode::KeyA),
        0.0,
        pressed(KeyCode::ArrowDown, KeyCode::KeyS) - pressed(KeyCode::ArrowUp, KeyCode::KeyW),
    );
    let step = direction.normalize_or_zero() * WALK_SPEED * time.delta_secs();
    let blocked = |position: Vec3| {
        solids.iter().any(|(transform, solid)| {
            let offset = (position - transform.translation).abs();
            let reach = solid.0 + Vec3::new(LISTENER_RADIUS, 0.0, LISTENER_RADIUS);
            offset.x < reach.x && offset.y < reach.y && offset.z < reach.z
        })
    };
    // Moving one axis at a time lets the listener slide along a wall. A listener caught by a
    // closing door may always move, so it can step out again.
    for axis_step in [Vec3::new(step.x, 0.0, 0.0), Vec3::new(0.0, 0.0, step.z)] {
        let position = listener.translation;
        let candidate = position + axis_step;
        if !blocked(candidate) || blocked(position) {
            listener.translation = candidate;
        }
    }
}

/// Shows or hides the rays with V.
fn toggle_rays(
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut draw: ResMut<'_, RaytracedAudioDebugDraw3d>,
) {
    if keys.just_pressed(KeyCode::KeyV) {
        draw.enabled = !draw.enabled;
    }
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

/// Writes the speaker's muffling and the room's echo under the explanation.
fn update_hud(
    door: Res<'_, Door>,
    trace: Res<'_, RaytracedAudioListenerTrace3d>,
    speaker: Single<'_, '_, Option<&RaytracedAudioRayResponse3d>, With<Speaker>>,
    mut hud: Single<'_, '_, &mut Text, With<Hud>>,
) {
    let (Some(traced), Some(listener_trace)) = (*speaker, trace.trace()) else {
        return;
    };
    let response = traced.response();
    let filter = response.filter();
    let reverb = listener_trace.reverb();
    hud.0 = format!(
        "{INTRO}\n\n\
         Door          {}\n\
         Line of sight {}\n\
         Green rays    {} {:>3.0}%\n\
         Muffle        {} {:>3.0}%\n\
         Bass          {} {:.2}\n\
         Treble        {} {:.2}\n\
         Echo returns  {} {:>3.0}%\n\
         Decay (RT60)  {:.2} s\n\
         Outdoors      {:>3.0}%",
        if door.entity.is_some() {
            "CLOSED"
        } else {
            "open"
        },
        if response.is_direct_visible() {
            "yes"
        } else {
            "no"
        },
        meter(response.discovered_fraction() * 2.0),
        response.discovered_fraction() * 100.0,
        meter(response.muffle_strength()),
        response.muffle_strength() * 100.0,
        meter(filter.gain_lf()),
        filter.gain_lf(),
        meter(filter.gain_hf()),
        filter.gain_hf(),
        meter(reverb.return_fraction()),
        reverb.return_fraction() * 100.0,
        reverb.decay_time_s(),
        reverb.outdoor_fraction() * 100.0,
    );
}
