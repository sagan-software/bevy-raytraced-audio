//! Muffling around a door: open it and the music is clear, close it and only the bass gets through.
//!
//! You (the blue circle) stand in the left room. A music speaker (the red circle) plays in the
//! right room. Every frame the listener fires sound rays that bounce off the walls; the white
//! lines show them travelling, slowed down so you can watch. When a bounce point can see the
//! speaker, a green line connects them: that sound reaches you around corners, so more green
//! means a clearer sound. Orange lines go straight through the walls, where the treble is lost.
//!
//! Try this: press Space to close the door and listen to the music turn dull and bassy, then
//! walk around with the arrow keys or WASD to see how line of sight changes the green rays.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale, Volume},
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

/// Wall thickness in meters.
const WALL: f32 = 0.2;
/// Half the size of each room in meters; the two rooms share the wall at `x = 0`.
const ROOM_HALF: f32 = 3.0;
/// Half the height of the doorway in meters.
const DOOR_HALF: f32 = 0.8;
/// Listener walking speed in meters per second.
const WALK_SPEED: f32 = 3.0;
/// Listener radius in meters, used for drawing and collisions.
const LISTENER_RADIUS: f32 = 0.25;
/// Speed at which the rays are drawn; real sound travels at 343 m/s.
const RAY_SPEED: f32 = 15.0;
/// Static part of the on-screen explanation.
const INTRO: &str = "MUFFLING AROUND A DOOR\n\n\
You (blue) are in the left room. Music\n\
(red) plays in the right room.\n\n\
White: rays leaving you, bouncing off walls.\n\
Green: a bounce point that can see the\n\
  speaker. More green = clearer music.\n\
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
                        title: "Bevy ray-traced audio: muffling around a door".to_owned(),
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
            RaytracedAudio2dPlugin::default().with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(128)
                    .expect("ray count is positive")
                    .with_max_bounces(6),
            ),
            RaytracedAudio2dDebugPlugin,
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .insert_resource(RaytracedAudioDebugDraw2d {
            kinds: RayKindMask::NONE
                .with(RayKind::Primary)
                .with(RayKind::Occlusion)
                .with(RayKind::Permeation),
            ray_speed_m_per_s: RAY_SPEED,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_door, move_listener, toggle_rays, update_hud),
        )
        .run();
}

/// Marks a solid wall; its sprite size doubles as its collision box.
#[derive(Component)]
struct Wall;

/// Marks the looping music speaker.
#[derive(Component)]
struct Speaker;

/// Marks the text panel.
#[derive(Component)]
struct Hud;

/// The door entity while the door is closed, and the sound it makes.
#[derive(Resource)]
struct Door {
    /// The closed door's wall entity, or `None` while the door is open.
    entity: Option<Entity>,
    /// Sound played when the door closes.
    slam: Handle<AudioSource>,
}

/// A thin plaster wall: it reflects most sound and lets some bass through.
fn thin_wall() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.08, 0.06).expect("bounded absorption"))
        .try_with_transmission(BandGain::try_new(0.55, 0.35, 0.12).expect("bounded transmission"))
        .expect("the material stays within its energy budget")
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
                    RaytracedAudioSurface2d::new(*start, *end, thin_wall())
                        .expect("wall faces have distinct finite endpoints"),
                    Transform::default(),
                ));
            }
        })
        .id()
}

/// Spawns the door that fills the doorway.
fn spawn_door(commands: &mut Commands<'_, '_>) -> Entity {
    spawn_wall(
        commands,
        Vec2::ZERO,
        Vec2::new(WALL, DOOR_HALF * 2.0),
        Color::srgb(0.6, 0.38, 0.2),
    )
}

/// Builds the two rooms, the listener, the speaker, the camera, and the text panel.
fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            // Always shows the rooms plus room on the left for the text panel.
            scaling_mode: ScalingMode::AutoMin {
                min_width: 24.0,
                min_height: 8.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(-4.5, 0.0, 0.0),
    ));

    let wall_color = Color::srgb(0.75, 0.74, 0.7);
    let width = ROOM_HALF.mul_add(4.0, WALL);
    let side = ROOM_HALF.mul_add(2.0, WALL);
    let piece = ROOM_HALF - DOOR_HALF;
    for (center, size) in [
        (Vec2::new(0.0, ROOM_HALF), Vec2::new(width, WALL)),
        (Vec2::new(0.0, -ROOM_HALF), Vec2::new(width, WALL)),
        (Vec2::new(-ROOM_HALF * 2.0, 0.0), Vec2::new(WALL, side)),
        (Vec2::new(ROOM_HALF * 2.0, 0.0), Vec2::new(WALL, side)),
        // The wall between the rooms, split around the doorway.
        (
            Vec2::new(0.0, DOOR_HALF + piece / 2.0),
            Vec2::new(WALL, piece),
        ),
        (
            Vec2::new(0.0, -DOOR_HALF - piece / 2.0),
            Vec2::new(WALL, piece),
        ),
    ] {
        spawn_wall(&mut commands, center, size, wall_color);
    }
    commands.insert_resource(Door {
        entity: None,
        slam: asset_server.load("audio/door_close.ogg"),
    });

    commands.spawn((
        RaytracedAudioListener2d,
        SpatialListener::new(LISTENER_RADIUS * 2.0),
        Mesh2d(meshes.add(Circle::new(LISTENER_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.6, 1.0))),
        Transform::from_xyz(-ROOM_HALF, 0.0, 1.0),
    ));

    commands.spawn((
        Speaker,
        RaytracedAudioEmitter2d,
        // The player filters the music with the traced muffling, so you hear the walls.
        RaytracedAudioPlayer::new(asset_server.load("audio/music_loop.ogg")).with_reverb_send(0.5),
        PlaybackSettings::LOOP.with_spatial(true),
        Mesh2d(meshes.add(Circle::new(0.35))),
        MeshMaterial2d(materials.add(Color::srgb(0.95, 0.3, 0.2))),
        Transform::from_xyz(ROOM_HALF, 0.0, 1.0),
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
    mut door: ResMut<'_, Door>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }
    if let Some(entity) = door.entity.take() {
        commands.entity(entity).despawn();
    } else {
        door.entity = Some(spawn_door(&mut commands));
        commands.spawn((
            RaytracedAudioEmitter2d,
            RaytracedAudioPlayer::new(door.slam.clone()),
            PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_volume(Volume::Linear(0.8)),
            Transform::from_xyz(WALL, 0.0, 0.0),
        ));
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
    // closing door may always move, so it can step out again.
    for axis_step in [Vec2::new(step.x, 0.0), Vec2::new(0.0, step.y)] {
        let position = listener.translation.truncate();
        let candidate = position + axis_step;
        if !blocked(candidate) || blocked(position) {
            listener.translation = candidate.extend(listener.translation.z);
        }
    }
}

/// Shows or hides the rays with V.
fn toggle_rays(
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut draw: ResMut<'_, RaytracedAudioDebugDraw2d>,
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

/// Writes the speaker's live ray-trace results under the explanation.
fn update_hud(
    door: Res<'_, Door>,
    speaker: Single<'_, '_, Option<&RaytracedAudioRayResponse2d>, With<Speaker>>,
    mut hud: Single<'_, '_, &mut Text, With<Hud>>,
) {
    let Some(traced) = *speaker else {
        return;
    };
    let response = traced.response();
    let filter = response.filter();
    hud.0 = format!(
        "{INTRO}\n\n\
         Door          {}\n\
         Line of sight {}\n\
         Green rays    {} {:>3.0}%\n\
         Muffle        {} {:>3.0}%\n\
         Bass          {} {:.2}\n\
         Treble        {} {:.2}",
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
    );
}

#[cfg(test)]
mod tests {
    //! Checks the door toggle without a window.

    use super::{Door, Wall, toggle_door};
    use bevy::prelude::{App, ButtonInput, Handle, KeyCode, Update};

    /// Pressing Space closes the open door, and pressing it again opens it.
    #[test]
    fn space_toggles_the_door() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(Door {
                entity: None,
                slam: Handle::default(),
            })
            .add_systems(Update, toggle_door);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        let door = app
            .world()
            .resource::<Door>()
            .entity
            .expect("Space closes the door");
        assert!(app.world().get::<Wall>(door).is_some());

        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Space);
        keys.clear();
        keys.press(KeyCode::Space);
        app.update();
        assert!(app.world().resource::<Door>().entity.is_none());
        assert!(app.world().get_entity(door).is_err());
    }
}
