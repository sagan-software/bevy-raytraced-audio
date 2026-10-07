//! Walk a small forest scene and watch Bevy's direct and reflected acoustic paths.
//!
//! The scene uses shared low-detail meshes and a fixed set of explicit acoustic
//! triangles. WASD moves the listener, a held left mouse button orbits the camera,
//! the wheel changes its distance, B toggles path lines, and F1 toggles diagnostics.

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, SpatialScale},
    diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_raytraced_audio::{AcousticMaterial, BandAbsorption, BandGain};
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioReflectionPaths3d, RaytracedAudioResponse3d, RaytracedAudioSurface3d,
};

/// Width of one stone arch post along the world Z axis, in meters.
const ARCH_POST_WIDTH_METERS: f32 = 0.42;
/// Height of each stone arch post, in meters.
const ARCH_POST_HEIGHT_METERS: f32 = 2.9;
/// Center distance of each post from the arch opening, in meters.
const ARCH_POST_CENTER_Z_METERS: f32 = 2.15;
/// Stone lintel span along the world Z axis, in meters.
const ARCH_LINTEL_SPAN_METERS: f32 = 4.7;
/// Stone lintel height, in meters.
const ARCH_LINTEL_HEIGHT_METERS: f32 = 0.42;
/// Stone lintel base elevation, in meters.
const ARCH_LINTEL_BASE_Y_METERS: f32 = 2.74;
/// Forest acoustic floor width along world X, in meters.
const FOREST_FLOOR_WIDTH_METERS: f32 = 28.0;
/// Forest acoustic floor depth along world Z, in meters.
const FOREST_FLOOR_DEPTH_METERS: f32 = 18.0;
/// Maximum listener movement along the X axis in meters.
const LISTENER_X_LIMIT_METERS: f32 = 7.0;
/// Maximum listener movement along the Z axis in meters.
const LISTENER_Z_LIMIT_METERS: f32 = 5.5;
/// Horizontal listener movement speed in meters per second.
const LISTENER_SPEED_METERS_PER_SECOND: f32 = 4.0;
/// Mouse motion conversion from pixels to orbit radians.
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.003;
/// Trunk height for a unit-scaled tree, in meters.
const TREE_TRUNK_HEIGHT_METERS: f32 = 1.7;
/// Each canopy layer stores its cone radius, height, and offset above the scaled trunk top in meters.
const TREE_CANOPY_LAYERS_METERS: [(f32, f32, f32); 3] =
    [(1.2, 1.9, 0.65), (0.9, 1.6, 1.4), (0.62, 1.3, 2.0)];
/// Source sway angular speed, in radians per second.
const EMITTER_SWAY_RADIANS_PER_SECOND: f32 = 0.42;
/// Source travel across the arch opening, in meters.
const EMITTER_SWAY_RADIUS_METERS: f32 = 4.2;
/// Small source orbit radius along the world X axis, in meters.
const EMITTER_ORBIT_RADIUS_METERS: f32 = 0.55;
/// Source center position along world X, in meters.
const EMITTER_CENTER_X_METERS: f32 = 4.2;
/// Source center elevation above the ground, in meters.
const EMITTER_CENTER_Y_METERS: f32 = 1.0;
/// Source vertical motion amplitude, in meters.
const EMITTER_VERTICAL_AMPLITUDE_METERS: f32 = 0.14;
/// Vertical pulse rate relative to the sway phase, as a unitless multiplier.
const EMITTER_VERTICAL_PHASE_MULTIPLIER: f32 = 4.0;
/// Visible source size pulse relative to the sway phase, as a unitless multiplier.
const EMITTER_SIZE_PHASE_MULTIPLIER: f32 = 5.7;
/// Visible source scale variation, as a unitless fraction.
const EMITTER_SIZE_PULSE_AMPLITUDE: f32 = 0.08;

/// Starts the forest example with Bevy's normal renderer and spatial audio output.
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
                        title: "Bevy Raytraced Audio — Forest Walk".to_owned(),
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
                    default_spatial_scale: SpatialScale::new(1.0),
                    ..default()
                }),
        )
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            RaytracedAudio3dPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .init_resource::<SceneDisplay>()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.07, 0.075)))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                control_scene,
                animate_emitter,
                draw_acoustic_paths,
                update_scene_readouts,
            ),
        )
        .run();
}

/// Creates the forest, listener, moving chime, surfaces, camera, and in-scene help.
fn setup_scene(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.28, 0.19),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(FOREST_FLOOR_WIDTH_METERS, FOREST_FLOOR_DEPTH_METERS),
            ),
        ),
        MeshMaterial3d(floor_material),
    ));

    // A single shadow-free directional light keeps the procedural scene inexpensive.
    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.65, 0.0)),
    ));

    let trunk_mesh = meshes.add(Cylinder::new(0.22, 1.7).mesh().resolution(7));
    let canopy_meshes = TREE_CANOPY_LAYERS_METERS
        .map(|(radius, height, _)| meshes.add(Cone::new(radius, height).mesh().resolution(7)));
    let trunk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.16, 0.105),
        perceptual_roughness: 1.0,
        ..default()
    });
    let canopy_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.28, 0.19),
            perceptual_roughness: 1.0,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.35, 0.22),
            perceptual_roughness: 1.0,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.39, 0.25),
            perceptual_roughness: 1.0,
            ..default()
        }),
    ];
    spawn_forest(
        &mut commands,
        &trunk_mesh,
        &canopy_meshes,
        &trunk_material,
        &canopy_materials,
    );

    spawn_acoustic_environment(&mut commands, &mut meshes, &mut materials);

    let listener_mesh = meshes.add(
        Sphere::new(0.24)
            .mesh()
            .ico(2)
            .expect("valid listener mesh"),
    );
    let listener_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.95, 0.58),
        emissive: LinearRgba::new(0.12, 0.9, 0.42, 1.0),
        ..default()
    });
    commands.spawn((
        Name::new("Moving acoustic listener"),
        RaytracedAudioListener3d,
        SpatialListener::new(0.25),
        Mesh3d(listener_mesh),
        MeshMaterial3d(listener_material),
        Transform::from_xyz(-4.8, 0.9, 0.0),
    ));

    let emitter_mesh = meshes.add(Sphere::new(0.2).mesh().ico(2).expect("valid emitter mesh"));
    let emitter_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.58, 0.24),
        emissive: LinearRgba::new(1.0, 0.31, 0.055, 1.0),
        ..default()
    });
    commands.spawn((
        Name::new("Looping spatial chime"),
        RaytracedAudioEmitter3d,
        RaytracedAudioReflectionPaths3d::default(),
        AudioPlayer::new(asset_server.load("audio/bevy-raytraced-audio-chime.wav")),
        PlaybackSettings::LOOP.with_spatial(true),
        Mesh3d(emitter_mesh),
        MeshMaterial3d(emitter_material),
        Transform::from_xyz(4.2, 1.0, 0.0),
        MovingEmitter,
    ));

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Transform::from_xyz(-8.0, 7.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ForestCamera {
            yaw_radians: -0.68,
            pitch_radians: 0.42,
            distance_meters: 11.0,
        },
    ));

    spawn_help_overlay(&mut commands);
}

/// Creates the acoustic arch, floor, brush screen, and path stones.
fn spawn_acoustic_environment(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Matching each acoustic panel to its visible geometry keeps the arch opening clear.
    let stone_material = materials.add(Color::srgb(0.42, 0.39, 0.31));
    for post_center_z in [-ARCH_POST_CENTER_Z_METERS, ARCH_POST_CENTER_Z_METERS] {
        spawn_visual_box(
            commands,
            meshes,
            stone_material.clone(),
            Vec3::new(0.15, ARCH_POST_HEIGHT_METERS / 2.0, post_center_z),
            Vec3::new(0.42, ARCH_POST_HEIGHT_METERS, ARCH_POST_WIDTH_METERS),
        );
        spawn_acoustic_panel(
            commands,
            panel_triangles(ARCH_POST_WIDTH_METERS, ARCH_POST_HEIGHT_METERS),
            Transform::from_xyz(0.15, 0.0, post_center_z),
            stone_acoustic_material(),
        );
    }

    spawn_visual_box(
        commands,
        meshes,
        stone_material,
        Vec3::new(
            0.15,
            ARCH_LINTEL_BASE_Y_METERS + ARCH_LINTEL_HEIGHT_METERS / 2.0,
            0.0,
        ),
        Vec3::new(0.48, ARCH_LINTEL_HEIGHT_METERS, ARCH_LINTEL_SPAN_METERS),
    );
    spawn_acoustic_panel(
        commands,
        panel_triangles(ARCH_LINTEL_SPAN_METERS, ARCH_LINTEL_HEIGHT_METERS),
        Transform::from_xyz(0.15, ARCH_LINTEL_BASE_Y_METERS, 0.0),
        stone_acoustic_material(),
    );

    // The floor returns first-order reflection energy beneath the listener and emitter.
    spawn_acoustic_panel(
        commands,
        floor_triangles(FOREST_FLOOR_WIDTH_METERS, FOREST_FLOOR_DEPTH_METERS),
        Transform::default(),
        forest_floor_acoustic_material(),
    );

    // The brush screen uses a different material profile from the stone frame.
    let brush_material = materials.add(Color::srgb(0.15, 0.3, 0.17));
    spawn_visual_box(
        commands,
        meshes,
        brush_material,
        Vec3::new(-1.55, 0.6, 3.5),
        Vec3::new(0.16, 1.2, 1.8),
    );
    spawn_acoustic_panel(
        commands,
        panel_triangles(1.8, 1.2),
        Transform::from_xyz(-1.55, 0.0, 3.5),
        brush_acoustic_material(),
    );

    spawn_path_stones(commands, meshes, materials);
}

/// Adds a fixed set of shared-mesh low-poly trees around the open walking path.
fn spawn_forest(
    commands: &mut Commands<'_, '_>,
    trunk_mesh: &Handle<Mesh>,
    canopy_meshes: &[Handle<Mesh>; 3],
    trunk_material: &Handle<StandardMaterial>,
    canopy_materials: &[Handle<StandardMaterial>; 3],
) {
    // Tree positions leave the center line and acoustic arch clear for movement.
    let trees = [
        (-8.0, -5.5, 1.0),
        (-6.5, 4.8, 0.9),
        (-4.3, -6.8, 0.8),
        (-3.5, 5.8, 1.1),
        (-1.6, -6.0, 0.85),
        (0.8, 6.4, 1.05),
        (2.1, -6.4, 0.95),
        (4.2, 5.8, 0.9),
        (6.3, -5.6, 1.1),
        (8.0, 4.8, 0.9),
        (-9.3, 1.6, 1.0),
        (9.1, -1.2, 1.0),
    ];

    for (tree_index, (x, z, scale)) in trees.into_iter().enumerate() {
        let trunk_height = TREE_TRUNK_HEIGHT_METERS * scale;
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_material.clone()),
            Transform::from_xyz(x, trunk_height / 2.0, z)
                .with_scale(Vec3::new(scale, scale, scale)),
        ));
        // Reuse three rough cone meshes to create a distinct, layered pine silhouette.
        for (layer, ((_, _, center_offset), canopy_mesh)) in TREE_CANOPY_LAYERS_METERS
            .into_iter()
            .zip(canopy_meshes.iter())
            .enumerate()
        {
            let material_index = (tree_index + layer) % canopy_materials.len();
            let canopy_material = canopy_materials
                .get(material_index)
                .expect("the example always supplies three canopy materials");
            let center_y = center_offset.mul_add(scale, trunk_height);
            let x_offset = if layer == 1 { 0.12 * scale } else { 0.0 };
            commands.spawn((
                Mesh3d(canopy_mesh.clone()),
                MeshMaterial3d(canopy_material.clone()),
                Transform::from_xyz(x + x_offset, center_y, z).with_scale(Vec3::splat(scale)),
            ));
        }
    }
}

/// Adds one visible cuboid with the supplied shared material.
fn spawn_visual_box(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
    ));
}

/// Creates two triangles that cover a vertical plane in local XZ coordinates.
fn panel_triangles(width_meters: f32, height_meters: f32) -> [[Vec3; 3]; 2] {
    let left_z = -width_meters / 2.0;
    let right_z = width_meters / 2.0;
    let bottom_left = Vec3::new(0.0, 0.0, left_z);
    let bottom_right = Vec3::new(0.0, 0.0, right_z);
    let top_left = Vec3::new(0.0, height_meters, left_z);
    let top_right = Vec3::new(0.0, height_meters, right_z);

    [
        [bottom_left, top_left, top_right],
        [bottom_left, top_right, bottom_right],
    ]
}

/// Creates two upward-facing triangles over the acoustic floor bounds.
fn floor_triangles(width_meters: f32, depth_meters: f32) -> [[Vec3; 3]; 2] {
    let left_x = -width_meters / 2.0;
    let right_x = width_meters / 2.0;
    let near_z = -depth_meters / 2.0;
    let far_z = depth_meters / 2.0;
    let near_left = Vec3::new(left_x, 0.0, near_z);
    let near_right = Vec3::new(right_x, 0.0, near_z);
    let far_left = Vec3::new(left_x, 0.0, far_z);
    let far_right = Vec3::new(right_x, 0.0, far_z);

    [
        [near_left, far_left, far_right],
        [near_left, far_right, near_right],
    ]
}

/// Adds explicit acoustic triangles that follow the supplied world transform.
fn spawn_acoustic_panel(
    commands: &mut Commands<'_, '_>,
    triangles: [[Vec3; 3]; 2],
    transform: Transform,
    material: AcousticMaterial,
) {
    for vertices in triangles {
        let surface = RaytracedAudioSurface3d::new(vertices, material)
            .expect("the example's fixed acoustic panels have nonzero area");
        commands.spawn((surface, transform));
    }
}

/// Defines stone absorption and small frequency-dependent amplitude transmission.
fn stone_acoustic_material() -> AcousticMaterial {
    let absorption = BandAbsorption::try_new(0.14, 0.22, 0.36)
        .expect("the example's fixed stone coefficients are valid");
    let transmission = BandGain::try_new(0.18, 0.12, 0.06)
        .expect("the example's fixed stone transmission is valid");
    AcousticMaterial::new(absorption)
        .try_with_transmission(transmission)
        .expect("the example's stone energy profile is valid")
}

/// Defines soft brush absorption and greater high-frequency transmission.
fn brush_acoustic_material() -> AcousticMaterial {
    let absorption = BandAbsorption::try_new(0.35, 0.52, 0.68)
        .expect("the example's fixed brush coefficients are valid");
    let transmission = BandGain::try_new(0.42, 0.3, 0.18)
        .expect("the example's fixed brush transmission is valid");
    AcousticMaterial::new(absorption)
        .try_with_transmission(transmission)
        .expect("the example's brush energy profile is valid")
}

/// Defines a mildly absorptive ground that returns visible first-order paths.
fn forest_floor_acoustic_material() -> AcousticMaterial {
    let absorption = BandAbsorption::try_new(0.18, 0.32, 0.48)
        .expect("the example's fixed ground coefficients are valid");
    AcousticMaterial::new(absorption)
}

/// Places a short stone path with a shared mesh and material handle.
fn spawn_path_stones(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let stone_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.85, 0.12, 0.55)));
    let stone_material = materials.add(Color::srgb(0.48, 0.43, 0.33));
    let stones = [
        (-4.0, 0.5, 0.0),
        (-3.0, -0.45, 0.15),
        (-2.0, 0.55, 0.3),
        (-1.0, -0.55, -0.2),
        (1.0, 0.55, 0.2),
        (2.0, -0.5, -0.1),
        (3.0, 0.55, 0.25),
        (4.0, -0.4, 0.0),
    ];
    for (x, z, angle) in stones {
        commands.spawn((
            Mesh3d(stone_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_xyz(x, 0.08, z).with_rotation(Quat::from_rotation_y(angle)),
        ));
    }
}

/// Places compact keyboard help and current path measurements over the Bevy canvas.
fn spawn_help_overlay(commands: &mut Commands<'_, '_>) {
    commands.spawn((
        Text::new("WASD move | hold left mouse to orbit | wheel zoom\nB paths | F1 diagnostics | Esc release cursor"),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.91, 0.94, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: px(14),
            left: px(14),
            padding: UiRect::all(px(11)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.065, 0.06, 0.86)),
    ));
    commands.spawn((
        Text::new("Direct path: waiting for the first trace"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.73, 0.91, 0.84)),
        Node {
            position_type: PositionType::Absolute,
            right: px(14),
            bottom: px(14),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.065, 0.06, 0.86)),
        PathReadout,
    ));
    commands.spawn((
        Text::new("FPS: measuring"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.7, 0.38)),
        Node {
            position_type: PositionType::Absolute,
            top: px(14),
            right: px(14),
            padding: UiRect::all(px(9)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.065, 0.06, 0.86)),
        FpsReadout,
    ));
}

/// Moves the listener, changes the camera orbit, and toggles the visual overlays.
fn control_scene(
    time: Res<'_, Time>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mouse_buttons: Res<'_, ButtonInput<MouseButton>>,
    mouse_motion: Res<'_, AccumulatedMouseMotion>,
    mut wheel_events: MessageReader<'_, '_, MouseWheel>,
    mut cursor: Single<'_, '_, &mut CursorOptions>,
    mut listener: Single<
        '_,
        '_,
        &mut Transform,
        (With<RaytracedAudioListener3d>, Without<ForestCamera>),
    >,
    mut camera: Single<'_, '_, (&mut Transform, &mut ForestCamera), With<ForestCamera>>,
    mut display: ResMut<'_, SceneDisplay>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        display.show_paths = !display.show_paths;
    }
    if keys.just_pressed(KeyCode::F1) {
        display.show_diagnostics = !display.show_diagnostics;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    if mouse_buttons.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    // Mouse deltas are already accumulated for the frame, so they do not use delta time.
    if mouse_buttons.pressed(MouseButton::Left) {
        camera.1.yaw_radians =
            (-ORBIT_RADIANS_PER_PIXEL).mul_add(mouse_motion.delta.x, camera.1.yaw_radians);
        camera.1.pitch_radians = (-ORBIT_RADIANS_PER_PIXEL)
            .mul_add(mouse_motion.delta.y, camera.1.pitch_radians)
            .clamp(0.12, 1.05);
    }
    for wheel in wheel_events.read() {
        camera.1.distance_meters = (-0.7_f32)
            .mul_add(wheel.y, camera.1.distance_meters)
            .clamp(6.0, 18.0);
    }

    // Movement uses world XZ axes so the listener can walk around either end of the wall.
    let mut movement = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        movement.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        movement.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        movement.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        movement.z += 1.0;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        movement *= 1.6;
    }
    listener.translation +=
        movement.normalize_or_zero() * LISTENER_SPEED_METERS_PER_SECOND * time.delta_secs();
    listener.translation.x = listener
        .translation
        .x
        .clamp(-LISTENER_X_LIMIT_METERS, LISTENER_X_LIMIT_METERS);
    listener.translation.z = listener
        .translation
        .z
        .clamp(-LISTENER_Z_LIMIT_METERS, LISTENER_Z_LIMIT_METERS);

    // The orbit camera follows the listener while preserving its chosen distance.
    let focus = listener.translation + Vec3::Y * 0.3;
    let distance = camera.1.distance_meters;
    let pitch = camera.1.pitch_radians;
    let yaw = camera.1.yaw_radians;
    let camera_offset = Vec3::new(
        distance * yaw.sin() * pitch.cos(),
        distance * pitch.sin(),
        distance * yaw.cos() * pitch.cos(),
    );
    camera.0.translation = focus + camera_offset;
    camera.0.look_at(focus, Vec3::Y);
}

/// Marks the single audio source that moves across the forest stone arch.
#[derive(Component)]
struct MovingEmitter;

/// Moves the source through the arch posts and pulses its visible size.
fn animate_emitter(
    time: Res<'_, Time>,
    mut emitter: Single<'_, '_, &mut Transform, With<MovingEmitter>>,
) {
    let phase = time.elapsed_secs() * EMITTER_SWAY_RADIANS_PER_SECOND;
    emitter.translation.x = phase
        .cos()
        .mul_add(EMITTER_ORBIT_RADIUS_METERS, EMITTER_CENTER_X_METERS);
    emitter.translation.y = (phase * EMITTER_VERTICAL_PHASE_MULTIPLIER)
        .sin()
        .mul_add(EMITTER_VERTICAL_AMPLITUDE_METERS, EMITTER_CENTER_Y_METERS);
    emitter.translation.z = phase.sin() * EMITTER_SWAY_RADIUS_METERS;
    let pulse = (phase * EMITTER_SIZE_PHASE_MULTIPLIER)
        .sin()
        .mul_add(EMITTER_SIZE_PULSE_AMPLITUDE, 1.0);
    emitter.scale = Vec3::splat(pulse);
}

/// Stores the camera's orbit pose and the listener's current view distance.
#[derive(Component)]
struct ForestCamera {
    /// Horizontal orbit angle in radians.
    yaw_radians: f32,
    /// Vertical orbit angle in radians.
    pitch_radians: f32,
    /// Distance from the listener focus point in meters.
    distance_meters: f32,
}

/// Stores toggles for the acoustic-path and frame-diagnostic overlays.
#[derive(Resource)]
struct SceneDisplay {
    /// Whether the direct path and first-order reflection lines are visible.
    show_paths: bool,
    /// Whether the frame-rate diagnostic is visible.
    show_diagnostics: bool,
}

impl Default for SceneDisplay {
    /// Starts with path lines visible and performance details hidden.
    fn default() -> Self {
        Self {
            show_paths: true,
            show_diagnostics: false,
        }
    }
}

/// Marks the text node that shows direct response and reflection-path count.
#[derive(Component)]
struct PathReadout;

/// Marks the optional frame-rate diagnostic text node.
#[derive(Component)]
struct FpsReadout;

/// Draws the direct path and first-order reflections from the current solver result.
fn draw_acoustic_paths(
    mut gizmos: Gizmos<'_, '_>,
    display: Res<'_, SceneDisplay>,
    listener: Single<'_, '_, &GlobalTransform, With<RaytracedAudioListener3d>>,
    emitter: Single<
        '_,
        '_,
        (
            &GlobalTransform,
            &RaytracedAudioResponse3d,
            &RaytracedAudioReflectionPaths3d,
        ),
        With<MovingEmitter>,
    >,
) {
    if !display.show_paths {
        return;
    }

    let source_position = emitter.0.translation();
    let listener_position = listener.translation();
    let response = emitter.1.response();
    let gain = response.direct.gain();
    let mean_gain = (gain.low() + gain.mid() + gain.high()) / 3.0;
    let direct_color = if mean_gain > 0.92 {
        Color::srgb(0.18, 0.95, 0.59)
    } else if mean_gain > 0.12 {
        Color::srgb(1.0, 0.65, 0.25)
    } else {
        Color::srgb(1.0, 0.25, 0.2)
    };
    gizmos.line(listener_position, source_position, direct_color);

    // These cyan lines show geometric reflection paths; they are not audible taps.
    let reflection_color = Color::srgb(0.16, 0.83, 1.0);
    for path in emitter.2.paths() {
        let reflection_point = render_point(path.reflection_point());
        gizmos.line(source_position, reflection_point, reflection_color);
        gizmos.line(reflection_point, listener_position, reflection_color);
    }
}

/// Converts validated solver meters into Bevy's `f32` render coordinates.
#[expect(
    clippy::cast_possible_truncation,
    reason = "The renderer uses f32 coordinates while the solver stores meter values as f64."
)]
const fn render_point(point: bevy_raytraced_audio::SolverPoint3d) -> Vec3 {
    Vec3::new(point.x_m() as f32, point.y_m() as f32, point.z_m() as f32)
}

/// Updates the response labels and the optional frame-rate diagnostic.
fn update_scene_readouts(
    emitter: Single<
        '_,
        '_,
        (&RaytracedAudioResponse3d, &RaytracedAudioReflectionPaths3d),
        With<MovingEmitter>,
    >,
    display: Res<'_, SceneDisplay>,
    diagnostics: Res<'_, DiagnosticsStore>,
    mut path_readout: Single<'_, '_, &mut Text, (With<PathReadout>, Without<FpsReadout>)>,
    mut fps_readout: Single<'_, '_, (&mut Text, &mut Visibility), With<FpsReadout>>,
) {
    let response = emitter.0.response();
    let direct = response.direct;
    let gain = direct.gain();
    let path_state = if direct.is_occluded() {
        "attenuated"
    } else {
        "clear"
    };
    let paths = emitter.1.paths().len();
    path_readout.0 = format!(
        "Direct path: {path_state} | reflection paths: {paths}\nLow {:.0}% | Mid {:.0}% | High {:.0}%",
        gain.low() * 100.0,
        gain.mid() * 100.0,
        gain.high() * 100.0,
    );

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::smoothed)
        .unwrap_or(0.0);
    fps_readout.0.0 = format!("FPS: {fps:.0} | CPU acoustic queries");
    *fps_readout.1 = if display.show_diagnostics {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

#[cfg(test)]
mod tests {
    use super::{
        ARCH_LINTEL_HEIGHT_METERS, ARCH_LINTEL_SPAN_METERS, ARCH_POST_HEIGHT_METERS,
        ARCH_POST_WIDTH_METERS, FOREST_FLOOR_DEPTH_METERS, FOREST_FLOOR_WIDTH_METERS, ForestCamera,
        MovingEmitter, SceneDisplay, animate_emitter, brush_acoustic_material, control_scene,
        floor_triangles, forest_floor_acoustic_material, panel_triangles,
    };
    use bevy::{
        input::mouse::{AccumulatedMouseMotion, MouseWheel},
        prelude::{App, ButtonInput, KeyCode, MouseButton, Time, Transform, Update, Vec3},
        window::CursorOptions,
    };
    use bevy_raytraced_audio_3d::RaytracedAudioListener3d;
    use std::time::Duration;

    /// Keeps each fixed panel's two triangles joined at the same diagonal.
    #[test]
    fn panel_triangles_share_one_diagonal_and_cover_the_requested_bounds() {
        let triangles = panel_triangles(ARCH_POST_WIDTH_METERS, ARCH_POST_HEIGHT_METERS);
        let first = triangles[0];
        let second = triangles[1];

        assert_eq!(first[0], second[0]);
        assert_eq!(first[2], second[1]);
        assert_eq!(first[0].y, 0.0);
        assert_eq!(first[1].y, ARCH_POST_HEIGHT_METERS);
        assert_eq!(first[0].z, -ARCH_POST_WIDTH_METERS / 2.0);
        assert_eq!(first[2].z, ARCH_POST_WIDTH_METERS / 2.0);
        assert_eq!(second[2], Vec3::new(0.0, 0.0, ARCH_POST_WIDTH_METERS / 2.0));
        assert!(ARCH_LINTEL_SPAN_METERS > ARCH_POST_WIDTH_METERS * 2.0);
        assert!(ARCH_LINTEL_HEIGHT_METERS > 0.0);
    }

    /// Keeps the acoustic floor flat and bounded by the rendered forest plane.
    #[test]
    fn floor_triangles_share_a_diagonal_and_match_the_scene_bounds() {
        let triangles = floor_triangles(FOREST_FLOOR_WIDTH_METERS, FOREST_FLOOR_DEPTH_METERS);
        let first = triangles[0];
        let second = triangles[1];

        assert_eq!(first[0], second[0]);
        assert_eq!(first[2], second[1]);
        assert!(first.into_iter().all(|vertex| vertex.y == 0.0));
        assert_eq!(first[0].x, -FOREST_FLOOR_WIDTH_METERS / 2.0);
        assert_eq!(first[2].x, FOREST_FLOOR_WIDTH_METERS / 2.0);
        assert_eq!(first[0].z, -FOREST_FLOOR_DEPTH_METERS / 2.0);
        assert_eq!(first[2].z, FOREST_FLOOR_DEPTH_METERS / 2.0);
        assert_eq!(second[2].z, -FOREST_FLOOR_DEPTH_METERS / 2.0);
        let normal = (first[1] - first[0]).cross(first[2] - first[0]);
        assert!(normal.y > 0.0);
    }

    /// Keeps the moving sound source visibly crossing both sides of the stone arch.
    #[test]
    fn emitter_crosses_the_acoustic_arch_during_its_loop() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .add_systems(Update, animate_emitter);
        let emitter = app
            .world_mut()
            .spawn((Transform::from_xyz(4.2, 1.0, 0.0), MovingEmitter))
            .id();
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs(4));

        app.update();

        let transform = app
            .world()
            .get::<Transform>(emitter)
            .expect("the moving emitter keeps its transform");
        assert!(transform.translation.z.abs() >= 3.0);
    }

    /// Verifies brush material energy remains within the validated absorption budget.
    #[test]
    fn brush_material_coefficients_fit_the_incident_energy_budget() {
        let material = brush_acoustic_material();
        let absorption = material.absorption();
        let transmission = material.transmission();
        let absorption = [absorption.low(), absorption.mid(), absorption.high()];
        let transmission = [transmission.low(), transmission.mid(), transmission.high()];

        for (absorption, transmission) in absorption.into_iter().zip(transmission) {
            assert!(absorption + transmission * transmission <= 1.0);
        }
    }

    /// Verifies the ground has partial absorption and returns reflection energy.
    #[test]
    fn floor_material_retains_energy_for_reflection_paths() {
        let material = forest_floor_acoustic_material();
        let absorption = material.absorption();
        let transmission = material.transmission();

        for (absorption, transmission) in [
            (absorption.low(), transmission.low()),
            (absorption.mid(), transmission.mid()),
            (absorption.high(), transmission.high()),
        ] {
            assert!((0.0..1.0).contains(&absorption));
            assert_eq!(transmission, 0.0);
            assert!(1.0 - absorption > 0.0);
        }
    }

    /// Initializes the controls with distinct listener and camera transforms.
    #[test]
    fn control_system_accepts_disjoint_listener_and_camera_queries() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(AccumulatedMouseMotion::default())
            .insert_resource(SceneDisplay::default())
            .insert_resource(Time::<()>::default())
            .add_message::<MouseWheel>()
            .add_systems(Update, control_scene);
        app.world_mut().spawn(CursorOptions::default());
        app.world_mut()
            .spawn((Transform::default(), RaytracedAudioListener3d));
        app.world_mut().spawn((
            Transform::default(),
            ForestCamera {
                yaw_radians: 0.0,
                pitch_radians: 0.5,
                distance_meters: 10.0,
            },
        ));

        app.update();
    }

    /// Moves the listener and its orbit camera when the user holds forward.
    #[test]
    fn forward_input_moves_listener_and_orbit_camera() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(AccumulatedMouseMotion::default())
            .insert_resource(SceneDisplay::default())
            .insert_resource(Time::<()>::default())
            .add_message::<MouseWheel>()
            .add_systems(Update, control_scene);
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs(1));
        let listener = app
            .world_mut()
            .spawn((
                Transform::from_xyz(-4.0, 0.9, 0.0),
                RaytracedAudioListener3d,
            ))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ForestCamera {
                    yaw_radians: 0.0,
                    pitch_radians: 0.5,
                    distance_meters: 10.0,
                },
            ))
            .id();
        app.world_mut().spawn(CursorOptions::default());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);

        app.update();

        let listener_transform = app
            .world()
            .get::<Transform>(listener)
            .expect("the listener keeps its transform");
        let camera_transform = app
            .world()
            .get::<Transform>(camera)
            .expect("the camera keeps its transform");
        assert_eq!(listener_transform.translation.x, 0.0);
        assert_eq!(camera_transform.translation.x, 0.0);
        assert!(camera_transform.translation.y > 0.0);
    }
}
