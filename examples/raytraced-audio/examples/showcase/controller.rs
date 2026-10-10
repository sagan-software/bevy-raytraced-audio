//! Mouse-facing third-person movement, first-person ears and traversable stairs/terrain.

use super::world::{Solid, Village, WalkSurface, ground_height};
use bevy::{
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_raytraced_audio_3d::RaytracedAudioListener3d;

/// Height of the ears above the character's feet.
pub(super) const EYE: f32 = 1.55;
/// Body clearance in narrow doorways.
pub(super) const RADIUS: f32 = 0.23;
/// Body marker: position is always the feet, forward is local -Z.
#[derive(Component)]
pub(super) struct Player;
/// Child containing the visible humanoid, hidden only in first person.
#[derive(Component)]
pub(super) struct Body;
/// Single follow camera.
#[derive(Component)]
pub(super) struct FollowCamera;
/// One leg's neutral pose and phase.
#[derive(Component)]
pub(super) struct Leg {
    /// Opposite sign for each leg.
    pub(super) side: f32,
}
/// Camera and accumulated locomotion state.
#[derive(Resource)]
pub(super) struct View {
    /// Whether camera sits at the player's ears.
    pub(super) first_person: bool,
    /// Character heading about world Y.
    pub(super) yaw: f32,
    /// First-person head tilt.
    pub(super) pitch: f32,
    /// Independent overhead camera orbit.
    pub(super) orbit: f32,
    /// Overhead zoom multiplier.
    pub(super) zoom: f32,
    /// Resume mouse-facing only after a new mouse movement following a preset jump.
    pub(super) aim_active: bool,
    /// Distance walked, used for footsteps and leg animation.
    pub(super) distance: f32,
}
impl Default for View {
    /// Faces into the speech house from its front garden.
    fn default() -> Self {
        Self {
            first_person: false,
            yaw: 0.0,
            pitch: 0.0,
            orbit: 0.0,
            zoom: 1.0,
            distance: 0.0,
            aim_active: true,
        }
    }
}

/// Installs the player, one listener and the independent follow camera.
pub(super) fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    let root = commands
        .spawn((
            Player,
            Transform::from_xyz(-9.0, 0.0, 5.0),
            Visibility::default(),
        ))
        .id();
    let body = humanoid(
        &mut commands,
        &mut meshes,
        &mut materials,
        Color::srgb(0.08, 0.48, 0.75),
    );
    commands.entity(body).insert((Body, ChildOf(root)));
    commands.spawn((
        RaytracedAudioListener3d,
        SpatialListener::new(0.3),
        Transform::from_xyz(0.0, EYE, 0.0),
        ChildOf(root),
    ));
    commands.spawn((
        Camera3d::default(),
        FollowCamera,
        camera_pose(Vec3::new(-9.0, 0.0, 5.0), &View::default()),
    ));
}

/// A readable humanoid with face, hands, legs and colored left/right ear cups.
pub(super) fn humanoid(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    shirt: Color,
) -> Entity {
    let root = commands
        .spawn((Transform::default(), Visibility::default()))
        .id();
    let skin = Color::srgb(0.72, 0.49, 0.33);
    for (center, size, color) in [
        (
            Vec3::new(0.0, 1.02, 0.0),
            Vec3::new(0.43, 0.52, 0.24),
            shirt,
        ),
        (Vec3::new(0.0, 1.47, 0.0), Vec3::new(0.31, 0.35, 0.29), skin),
        (
            Vec3::new(0.0, 1.47, -0.17),
            Vec3::new(0.065, 0.07, 0.10),
            skin,
        ),
        (
            Vec3::new(-0.065, 1.54, -0.151),
            Vec3::new(0.045, 0.04, 0.015),
            Color::BLACK,
        ),
        (
            Vec3::new(0.065, 1.54, -0.151),
            Vec3::new(0.045, 0.04, 0.015),
            Color::BLACK,
        ),
        (
            Vec3::new(-0.19, 1.49, 0.0),
            Vec3::new(0.07, 0.16, 0.17),
            Color::srgb(0.05, 0.35, 1.0),
        ),
        (
            Vec3::new(0.19, 1.49, 0.0),
            Vec3::new(0.07, 0.16, 0.17),
            Color::srgb(1.0, 0.25, 0.15),
        ),
        (
            Vec3::new(-0.3, 0.98, 0.0),
            Vec3::new(0.13, 0.48, 0.17),
            shirt,
        ),
        (
            Vec3::new(0.3, 0.98, 0.0),
            Vec3::new(0.13, 0.48, 0.17),
            shirt,
        ),
        (Vec3::new(-0.3, 0.68, 0.0), Vec3::splat(0.13), skin),
        (Vec3::new(0.3, 0.68, 0.0), Vec3::splat(0.13), skin),
    ] {
        let part = super::world::cuboid(commands, meshes, materials, center, size, color);
        commands.entity(part).insert(ChildOf(root));
    }
    for side in [-1.0, 1.0] {
        let leg = super::world::cuboid(
            commands,
            meshes,
            materials,
            Vec3::new(side * 0.115, 0.39, 0.0),
            Vec3::new(0.17, 0.72, 0.19),
            Color::srgb(0.12, 0.17, 0.22),
        );
        commands.entity(leg).insert((Leg { side }, ChildOf(root)));
        let shoe = super::world::cuboid(
            commands,
            meshes,
            materials,
            Vec3::new(0.0, -0.30, -0.055),
            Vec3::new(0.19, 0.14, 0.31),
            Color::srgb(0.12, 0.10, 0.08),
        );
        commands.entity(shoe).insert(ChildOf(leg));
    }
    root
}

/// Eye-aligned first person; overhead heading depends only on camera orbit, never body yaw.
pub(super) fn camera_pose(feet: Vec3, view: &View) -> Transform {
    if view.first_person {
        Transform::from_translation(feet + Vec3::Y * EYE)
            .with_rotation(Quat::from_rotation_y(view.yaw) * Quat::from_rotation_x(view.pitch))
    } else {
        let offset = Quat::from_rotation_y(view.orbit)
            * Vec3::new(0.0, 14.0, if feet.y < -0.4 { 6.0 } else { 10.0 })
            * view.zoom;
        Transform::from_translation(feet + offset).looking_at(feet + Vec3::Y * 0.5, Vec3::Y)
    }
}

/// Faces the mouse's ground intersection in overhead mode; captured mouse looks in first person.
pub(super) fn look(
    keys: Res<'_, ButtonInput<KeyCode>>,
    motion: Res<'_, AccumulatedMouseMotion>,
    time: Res<'_, Time<Real>>,
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut view: ResMut<'_, View>,
    mut window: Query<'_, '_, (&Window, &mut CursorOptions), With<PrimaryWindow>>,
    camera: Query<'_, '_, (&Camera, &GlobalTransform), With<FollowCamera>>,
    mut player: Query<'_, '_, &mut Transform, With<Player>>,
    mut body: Query<'_, '_, &mut Visibility, With<Body>>,
) {
    let Ok((window, mut cursor)) = window.single_mut() else {
        return;
    };
    let Ok(mut player) = player.single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyF) {
        view.first_person = !view.first_person;
    }
    if keys.just_pressed(KeyCode::Escape) {
        view.first_person = false;
    }
    let grab = if view.first_person {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    if cursor.grab_mode != grab {
        cursor.grab_mode = grab;
    }
    if cursor.visible == view.first_person {
        cursor.visible = !view.first_person;
    }
    for mut visibility in &mut body {
        let desired = if view.first_person {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }
    let turn = f32::from(keys.pressed(KeyCode::KeyQ)) - f32::from(keys.pressed(KeyCode::KeyR));
    if view.first_person {
        view.yaw = (turn * time.delta_secs().min(0.1)).mul_add(1.8, view.yaw);
        if window.focused {
            view.yaw = motion.delta.x.mul_add(-0.003, view.yaw);
            view.pitch = motion.delta.y.mul_add(-0.003, view.pitch);
        }
        view.pitch = (f32::from(keys.pressed(KeyCode::PageUp))
            - f32::from(keys.pressed(KeyCode::PageDown)))
        .mul_add(time.delta_secs().min(0.1), view.pitch);
        view.pitch = view.pitch.clamp(-1.35, 1.35);
    } else {
        view.orbit = (turn * time.delta_secs().min(0.1)).mul_add(1.5, view.orbit);
        for event in wheel.read() {
            view.zoom = event.y.mul_add(-0.04, view.zoom).clamp(0.45, 1.6);
        }
        if motion.delta.length_squared() > 0.0 {
            view.aim_active = true;
        }
        if view.aim_active
            && let Some(cursor) = window.cursor_position()
            && let Ok((camera, pose)) = camera.single()
            && let Ok(ray) = camera.viewport_to_world(pose, cursor)
            && let Some(distance) =
                ray.intersect_plane(player.translation, InfinitePlane3d::new(Vec3::Y))
        {
            let direction = (ray.get_point(distance) - player.translation).xz();
            if direction.length_squared() > 0.04 {
                view.yaw = (-direction.x).atan2(-direction.y);
            }
        }
    }
    player.rotation = Quat::from_rotation_y(view.yaw);
}

/// Body/box test in the box's local coordinates, including rotated doors.
pub(super) fn overlaps(feet: Vec3, solid: Solid, pose: &GlobalTransform) -> bool {
    let local = pose
        .affine()
        .inverse()
        .transform_point3(feet + Vec3::Y * 0.85)
        - solid.center;
    if local.y.abs() >= solid.half.y + 0.78 {
        return false;
    }
    let nearest = local.xz().clamp(-solid.half.xz(), solid.half.xz());
    (local.xz() - nearest).length_squared() < RADIUS * RADIUS
}

/// Best reachable walking support, excluding terrain over the basement excavation.
pub(super) fn support_height(feet: Vec3, supports: &[WalkSurface]) -> f32 {
    let inside_basement = (6.0..14.0).contains(&feet.x) && (-6.0..2.0).contains(&feet.z);
    let mut height = if inside_basement {
        -2.5
    } else {
        ground_height(feet.x)
    };
    for surface in supports {
        if let Some(candidate) = surface.at(feet.xz())
            && candidate <= feet.y + 0.34
            && candidate > height
        {
            height = candidate;
        }
    }
    height
}

/// Moves in small substeps so walls and narrow door frames cannot be skipped on slow frames.
pub(super) fn walk(
    keys: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time<Real>>,
    mut view: ResMut<'_, View>,
    village: Res<'_, Village>,
    mut player: Query<'_, '_, &mut Transform, With<Player>>,
    solids: Query<'_, '_, (&Solid, &GlobalTransform)>,
) {
    let Ok(mut player) = player.single_mut() else {
        return;
    };
    let input = Vec2::new(
        f32::from(keys.pressed(KeyCode::KeyD)) - f32::from(keys.pressed(KeyCode::KeyA)),
        f32::from(keys.pressed(KeyCode::KeyS)) - f32::from(keys.pressed(KeyCode::KeyW)),
    )
    .normalize_or_zero();
    let yaw = if view.first_person {
        view.yaw
    } else {
        view.orbit
    };
    let direction = Quat::from_rotation_y(yaw) * Vec3::new(input.x, 0.0, input.y);
    let distance = time.delta_secs().min(0.15)
        * if keys.pressed(KeyCode::ShiftLeft) {
            4.6
        } else {
            2.8
        };
    let start = player.translation;
    for _ in 0..8 {
        for delta in [
            Vec3::X * direction.x * distance / 8.0,
            Vec3::Z * direction.z * distance / 8.0,
        ] {
            let mut next = player.translation + delta;
            next.x = next.x.clamp(-21.5, 21.5);
            next.z = next.z.clamp(-17.5, 17.5);
            let support = support_height(next, &village.supports);
            next.y = if support >= next.y {
                support
            } else {
                (next.y - distance / 8.0).max(support)
            };
            if !village.supports.iter().any(|surface| surface.blocks(next))
                && !solids
                    .iter()
                    .any(|(solid, pose)| overlaps(next, *solid, pose))
            {
                player.translation = next;
            }
        }
    }
    view.distance += player.translation.xz().distance(start.xz());
}

/// Keeps the camera aligned with the current frame's character pose.
pub(super) fn camera(
    view: Res<'_, View>,
    player: Query<'_, '_, &Transform, (With<Player>, Without<FollowCamera>)>,
    mut camera: Query<'_, '_, &mut Transform, With<FollowCamera>>,
) {
    if let Ok(player) = player.single()
        && let Ok(mut camera) = camera.single_mut()
    {
        *camera = camera_pose(player.translation, &view);
    }
}

/// Head pitch follows first-person looking, so elevation cues track the ears rather than the camera orbit.
pub(super) fn head_pose(
    view: Res<'_, View>,
    mut head: Query<'_, '_, &mut Transform, With<RaytracedAudioListener3d>>,
) {
    for mut head in &mut head {
        head.rotation = Quat::from_rotation_x(if view.first_person { view.pitch } else { 0.0 });
    }
}
