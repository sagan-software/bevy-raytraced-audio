//! Indoor, outdoor, and rain direction: open a window and the rain comes in through it.
//!
//! You (the blue circle) stand in a small house while it rains outside. Grey lines are rays that
//! escaped the house, through the door, a window, or because you are already outdoors. For each
//! escaped ray, a yellow line points at the last spot that could still see you: that is where the
//! rain enters. The big yellow arrow averages them into one direction, and the rain recording is
//! placed there. The more rays escape, the louder and brighter the rain; indoors it is a dull,
//! muffled rumble on the walls.
//!
//! Try this: press 1, 2, and 3 to open and close the front door and the two windows, and walk in
//! and out of the house with the arrow keys. With everything shut the rain goes quiet and dull;
//! open one window and the arrow and the rain swing toward it.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale, Volume},
    camera::ScalingMode,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::{
    AcousticMaterial, BandAbsorption, MuffleFilter, RayKind, RayTraceSettings,
};
use bevy_raytraced_audio_2d::{
    RayKindMask, RaytracedAudio2dDebugPlugin, RaytracedAudio2dPlugin, RaytracedAudioDebugDraw2d,
    RaytracedAudioListener2d, RaytracedAudioListenerTrace2d, RaytracedAudioPlayer,
    RaytracedAudioSurface2d,
};

/// Half the width of the house in meters.
const HOUSE_X: f32 = 5.0;
/// Half the depth of the house in meters.
const HOUSE_Y: f32 = 3.5;
/// Wall thickness in meters.
const WALL: f32 = 0.25;
/// Listener walking speed in meters per second.
const WALK_SPEED: f32 = 3.5;
/// Listener radius in meters, used for drawing and collisions.
const LISTENER_RADIUS: f32 = 0.25;
/// Farthest the listener may walk from the house centre, in meters.
const YARD: Vec2 = Vec2::new(9.0, 6.5);
/// Static part of the on-screen explanation.
const INTRO: &str = "INDOOR, OUTDOOR, AND RAIN DIRECTION\n\n\
You (blue) are in a house. It is raining.\n\n\
Grey: rays that escaped to the sky.\n\
Yellow: where each escaped ray last saw\n\
  you, so where the rain gets in.\n\
Arrow: the average rain direction. The\n\
  rain sound is placed along it.\n\
More escaped rays = louder, brighter\n\
  rain. Shut in, it is a dull rumble.\n\n\
1 / 2 / 3     door / west / north window\n\
Arrows, WASD  walk\n\
B             show the bounces too";

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
                        title: "Bevy ray-traced audio: indoor and outdoor ambience".to_owned(),
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
                    default_spatial_scale: SpatialScale::new(0.3),
                    ..default()
                }),
        )
        .add_plugins((
            RaytracedAudio2dPlugin::default().with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(160)
                    .expect("ray count is positive")
                    .with_max_bounces(6)
                    .try_with_escape_distance(30.0)
                    .expect("escape distance is positive"),
            ),
            RaytracedAudio2dDebugPlugin,
        ))
        .init_gizmo_group::<ArrowGizmos>()
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.12, 0.17, 0.13)))
        .insert_resource(RaytracedAudioDebugDraw2d {
            kinds: RayKindMask::NONE
                .with(RayKind::Escaped)
                .with(RayKind::Ambient),
            ray_speed_m_per_s: 15.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_keys,
                move_listener,
                update_rain,
                draw_arrow,
                update_hud,
            ),
        )
        .run();
}

/// Thick gizmo lines for the rain-direction arrow.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct ArrowGizmos;

/// Marks a solid wall; its sprite size doubles as its collision box.
#[derive(Component)]
struct Wall;

/// Marks the rain loop that follows the escaped rays.
#[derive(Component)]
struct Rain;

/// Marks the text panel.
#[derive(Component)]
struct Hud;

/// One door or window that can be opened and closed.
struct Opening {
    /// Name shown in the text panel.
    name: &'static str,
    /// Key that toggles it.
    key: KeyCode,
    /// Centre of the gap in meters.
    center: Vec2,
    /// Size of the gap in meters.
    size: Vec2,
    /// The wall entity filling the gap while closed, or `None` while open.
    filler: Option<Entity>,
}

/// Every door and window in the house.
#[derive(Resource)]
struct Openings(Vec<Opening>);

/// Painted brick: it reflects most sound and lets none through.
fn brick() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.05, 0.08, 0.1).expect("bounded absorption"))
}

/// Spawns a rectangular wall with a sprite and four acoustic faces.
fn spawn_wall(commands: &mut Commands<'_, '_>, center: Vec2, size: Vec2, color: Color) -> Entity {
    let half = size / 2.0;
    let corners = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        half,
        Vec2::new(-half.x, half.y),
    ];
    commands
        .spawn((
            Wall,
            Sprite::from_color(color, size),
            Transform::from_translation(center.extend(0.0)),
        ))
        .with_children(|wall| {
            for (start, end) in corners.iter().zip(corners.iter().cycle().skip(1)) {
                wall.spawn((
                    RaytracedAudioSurface2d::new(*start, *end, brick())
                        .expect("wall faces have distinct finite endpoints"),
                    Transform::default(),
                ));
            }
        })
        .id()
}

/// Spawns the filler that closes a door or window.
fn close(commands: &mut Commands<'_, '_>, opening: &Opening) -> Entity {
    spawn_wall(
        commands,
        opening.center,
        opening.size,
        Color::srgb(0.55, 0.75, 0.9),
    )
}

/// Builds the house, listener, rain, camera, and text panel.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
    mut gizmo_config: ResMut<'_, GizmoConfigStore>,
) {
    gizmo_config.config_mut::<ArrowGizmos>().0.line.width = 7.0;
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            // Always shows the yard plus room on the left for the text panel.
            scaling_mode: ScalingMode::AutoMin {
                min_width: 30.0,
                min_height: 14.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(-5.5, 0.0, 0.0),
    ));

    // The wooden floor shows where "indoors" is.
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.36, 0.27, 0.18),
            Vec2::new(HOUSE_X, HOUSE_Y) * 2.0,
        ),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    // Fixed wall pieces around the three gaps: door (south), west window, and north window.
    let color = Color::srgb(0.72, 0.5, 0.42);
    let (x, y) = (HOUSE_X, HOUSE_Y);
    let corner = x + WALL / 2.0;
    // A wall running along x from `from` to `to` at height `at`, and one running along y.
    let along_x = |from: f32, to: f32, at: f32| {
        (
            Vec2::new(f32::midpoint(from, to), at),
            Vec2::new(to - from, WALL),
        )
    };
    let along_y = |from: f32, to: f32, at: f32| {
        (
            Vec2::new(at, f32::midpoint(from, to)),
            Vec2::new(WALL, to - from),
        )
    };
    for (center, size) in [
        along_x(-corner, -0.7, -y),
        along_x(0.7, corner, -y),
        along_x(-corner, 1.0, y),
        along_x(3.0, corner, y),
        along_y(-y, -1.0, -x),
        along_y(1.0, y, -x),
        along_y(-y, y, x),
    ] {
        spawn_wall(&mut commands, center, size, color);
    }

    // The front door starts open; both windows start closed.
    let mut openings = vec![
        Opening {
            name: "Front door  ",
            key: KeyCode::Digit1,
            center: Vec2::new(0.0, -y),
            size: Vec2::new(1.4, WALL),
            filler: None,
        },
        Opening {
            name: "West window ",
            key: KeyCode::Digit2,
            center: Vec2::new(-x, 0.0),
            size: Vec2::new(WALL, 2.0),
            filler: None,
        },
        Opening {
            name: "North window",
            key: KeyCode::Digit3,
            center: Vec2::new(2.0, y),
            size: Vec2::new(2.0, WALL),
            filler: None,
        },
    ];
    for opening in openings.iter_mut().skip(1) {
        opening.filler = Some(close(&mut commands, opening));
    }
    commands.insert_resource(Openings(openings));

    commands.spawn((
        RaytracedAudioListener2d,
        SpatialListener::new(LISTENER_RADIUS * 2.0),
        Mesh2d(meshes.add(Circle::new(LISTENER_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(2.0, 1.0, 1.0),
    ));

    // The rain is not a traced emitter: it has no position of its own. `update_rain` places it
    // where the escaped rays say it comes from and sets its filter by hand.
    commands.spawn((
        Rain,
        RaytracedAudioPlayer::new(asset_server.load("audio/rain_loop.ogg")),
        PlaybackSettings::LOOP
            .with_spatial(true)
            .with_volume(Volume::Linear(0.8)),
        Transform::from_xyz(2.0, 1.0, 1.0),
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

/// Opens and closes the door and windows with 1, 2, and 3, and shows bounces with B.
fn handle_keys(
    mut commands: Commands<'_, '_>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut openings: ResMut<'_, Openings>,
    mut draw: ResMut<'_, RaytracedAudioDebugDraw2d>,
) {
    for opening in &mut openings.0 {
        if !keys.just_pressed(opening.key) {
            continue;
        }
        if let Some(filler) = opening.filler.take() {
            commands.entity(filler).despawn();
        } else {
            opening.filler = Some(close(&mut commands, opening));
        }
    }
    if keys.just_pressed(KeyCode::KeyB) {
        draw.kinds.toggle(RayKind::Primary);
    }
}

/// Walks the listener with the arrow keys or WASD, sliding along walls.
fn move_listener(
    keys: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time>,
    mut listener: Single<'_, '_, &mut Transform, With<RaytracedAudioListener2d>>,
    walls: Query<'_, '_, (&Transform, &Sprite), (With<Wall>, Without<RaytracedAudioListener2d>)>,
) {
    let pressed = |a: KeyCode, b: KeyCode| f32::from(keys.pressed(a) || keys.pressed(b));
    let direction = Vec2::new(
        pressed(KeyCode::ArrowRight, KeyCode::KeyD) - pressed(KeyCode::ArrowLeft, KeyCode::KeyA),
        pressed(KeyCode::ArrowUp, KeyCode::KeyW) - pressed(KeyCode::ArrowDown, KeyCode::KeyS),
    );
    let step = direction.normalize_or_zero() * WALK_SPEED * time.delta_secs();
    let blocked = |position: Vec2| {
        walls.iter().any(|(wall, sprite)| {
            let half = sprite.custom_size.unwrap_or_default() / 2.0 + LISTENER_RADIUS;
            let offset = (position - wall.translation.truncate()).abs();
            offset.x < half.x && offset.y < half.y
        })
    };
    // Moving one axis at a time lets the listener slide along a wall. A listener caught by a
    // closing window may always move, so it can step out again.
    for axis_step in [Vec2::new(step.x, 0.0), Vec2::new(0.0, step.y)] {
        let position = listener.translation.truncate();
        let candidate = (position + axis_step).clamp(-YARD, YARD);
        if !blocked(candidate) || blocked(position) {
            listener.translation = candidate.extend(listener.translation.z);
        }
    }
}

/// Returns the rain direction as a unit vector, if any escaped ray could see the listener.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the solver returns a unit vector in f64; render precision is plenty"
)]
fn rain_direction(trace: &RaytracedAudioListenerTrace2d) -> Option<Vec2> {
    let direction = trace.trace()?.ambient_direction()?;
    Some(Vec2::new(direction.x_m() as f32, direction.y_m() as f32))
}

/// Places the rain along the escaped-ray direction and opens its filter outdoors.
fn update_rain(
    trace: Res<'_, RaytracedAudioListenerTrace2d>,
    listener: Single<'_, '_, &Transform, (With<RaytracedAudioListener2d>, Without<Rain>)>,
    rain: Single<'_, '_, (&mut Transform, &RaytracedAudioPlayer), With<Rain>>,
) {
    let Some(traced) = trace.trace() else {
        return;
    };
    let (mut rain_transform, rain_player) = rain.into_inner();
    let outdoor = traced.reverb().outdoor_fraction();
    // Outdoors the rain is all around (no offset); indoors it comes from the openings.
    let offset = rain_direction(&trace).unwrap_or_default() * (1.0 - outdoor).sqrt() * 5.0;
    rain_transform.translation = (listener.translation.truncate() + offset).extend(1.0);

    // A shut house still hears a muffled rumble; every escaped ray adds volume and treble.
    let level = (0.2 + outdoor).min(1.0);
    let filter = MuffleFilter::try_new(level, level * outdoor.mul_add(0.9, 0.1))
        .unwrap_or(MuffleFilter::SILENT);
    rain_player.params().set_filter(filter);
    rain_player.params().set_reverb(traced.reverb(), 0.4);
}

/// Draws a big yellow arrow from the listener toward where the rain comes in.
fn draw_arrow(
    trace: Res<'_, RaytracedAudioListenerTrace2d>,
    listener: Single<'_, '_, &Transform, With<RaytracedAudioListener2d>>,
    mut gizmos: Gizmos<'_, '_, ArrowGizmos>,
) {
    let Some(direction) = rain_direction(&trace) else {
        return;
    };
    let start = listener.translation.truncate() + direction * LISTENER_RADIUS * 1.5;
    gizmos
        .arrow_2d(start, start + direction * 2.5, Color::srgb(1.0, 0.85, 0.15))
        .with_tip_length(0.7);
}

/// Names a unit direction like a compass, for example "north-west".
fn compass(direction: Vec2) -> String {
    // sin(22.5 degrees): beyond it, a direction counts toward that side.
    let threshold = 0.38;
    let north_south = if direction.y > threshold {
        "north"
    } else if direction.y < -threshold {
        "south"
    } else {
        ""
    };
    let east_west = if direction.x > threshold {
        "east"
    } else if direction.x < -threshold {
        "west"
    } else {
        ""
    };
    let joiner = if north_south.is_empty() || east_west.is_empty() {
        ""
    } else {
        "-"
    };
    format!("{north_south}{joiner}{east_west}")
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

/// Writes the live outdoor estimate and rain settings under the explanation.
fn update_hud(
    trace: Res<'_, RaytracedAudioListenerTrace2d>,
    openings: Res<'_, Openings>,
    rain: Single<'_, '_, &RaytracedAudioPlayer, With<Rain>>,
    mut hud: Single<'_, '_, &mut Text, With<Hud>>,
) {
    let Some(traced) = trace.trace() else {
        return;
    };
    let outdoor = traced.reverb().outdoor_fraction();
    let focus = traced.ambient_focus();
    let direction = rain_direction(&trace).map_or_else(
        || "nowhere (sealed in)".to_owned(),
        |direction| format!("the {}", compass(direction)),
    );
    let filter = rain.params().filter();
    let doors = openings
        .0
        .iter()
        .map(|opening| {
            let state = if opening.filler.is_some() {
                "closed"
            } else {
                "OPEN"
            };
            format!("{}  {state}", opening.name)
        })
        .collect::<Vec<_>>()
        .join("\n");
    hud.0 = format!(
        "{INTRO}\n\n{doors}\n\n\
         Outdoors     {} {:>3.0}%\n\
         Rain from    {direction}\n\
         Focus        {} {:>3.0}%\n\
         Rain bass    {} {:.2}\n\
         Rain treble  {} {:.2}",
        meter(outdoor),
        outdoor * 100.0,
        meter(focus),
        focus * 100.0,
        meter(filter.gain_lf()),
        filter.gain_lf(),
        meter(filter.gain_hf()),
        filter.gain_hf(),
    );
}
