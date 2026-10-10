//! Regression scenarios exercise the same openings, floors, movement and mixer as the village.

use super::{
    audio::{Kind, Mix},
    controller::{EYE, View, camera_pose, overlaps, support_height},
    world::{Module, Opening, Solid, Surface, WalkSurface, house_plan, plane, wall},
};
use bevy::prelude::{GlobalTransform, Quat, SpatialListener, Transform, Vec2, Vec3};
use bevy_raytraced_audio::{AcousticScene3d, BandGain, Emitter3d, Listener3d, Point3, Triangle3d};
use rodio::{buffer::SamplesBuffer, source::Spatial};
use std::num::{NonZeroU16, NonZeroU32};

/// Adds the exact thin-box planes used by the live modular scene.
fn add_module(scene: &mut AcousticScene3d, module: &Module) {
    for solid in &module.boxes {
        let points = plane(*solid).map(|point| {
            let p = module.pose.transform_point(point);
            Point3::try_new(p.x, p.y, p.z).unwrap()
        });
        for indices in [[0, 1, 2], [0, 2, 3]] {
            scene.add_triangle(
                Triangle3d::try_new(indices.map(|i| points[i]), module.surface.material()).unwrap(),
            );
        }
    }
}
/// Queries direct transmission at real listener and source heights.
fn gain(scene: &AcousticScene3d, source: Vec3, listener: Vec3) -> BandGain {
    scene
        .trace(
            Emitter3d::new(Point3::try_new(source.x, source.y, source.z).unwrap()),
            Listener3d::new(Point3::try_new(listener.x, listener.y, listener.z).unwrap()),
        )
        .direct
        .gain()
}

/// Open windows and doors are real holes; closing the leaf blocks precisely that aperture.
#[test]
fn opening_geometry_and_leaf_transmission_match() {
    for (opening, height, width, material) in [
        (Opening::Door, 1.4, 0.9, Surface::Wood),
        (Opening::Double, 1.4, 2.9, Surface::Wood),
        (Opening::Window, 1.4, 0.9, Surface::Glass),
    ] {
        let frame = wall(Transform::default(), opening);
        let mut scene = AcousticScene3d::default();
        add_module(&mut scene, &frame);
        let source = Vec3::new(-2.0, height, 0.0);
        let listener = Vec3::new(2.0, height, 0.0);
        assert_eq!(gain(&scene, source, listener), BandGain::UNITY);
        assert!(
            gain(
                &scene,
                source + Vec3::Z * (width / 2.0 + 0.15),
                listener + Vec3::Z * (width / 2.0 + 0.15)
            )
            .high()
                < 0.1
        );
        let leaf = Module {
            model: "",
            pose: Transform::default(),
            boxes: vec![Solid {
                center: Vec3::new(0.0, 1.2, 0.0),
                half: Vec3::new(0.055, 1.2, width / 2.0),
            }],
            surface: material,
            blocks: true,
            level: 0.0,
        };
        add_module(&mut scene, &leaf);
        assert_eq!(
            gain(&scene, source, listener),
            material.material().transmission()
        );
    }
}

/// The actual vertical house separates basement and upstairs sources with real horizontal floors.
#[test]
fn floors_muffle_sources_above_and_below() {
    let (modules, _) = house_plan(10.0, true);
    let mut scene = AcousticScene3d::default();
    for module in &modules {
        add_module(&mut scene, module);
    }
    let listener = Vec3::new(10.0, EYE, -2.0);
    for y in [-2.42, 2.58] {
        let response = gain(&scene, Vec3::new(10.0, y, -2.0), listener);
        assert!(
            response.high() < 0.06,
            "floor at source y={y}: {response:?}"
        );
        assert!(response.low() > response.high() * 5.0);
    }
    assert_eq!(
        gain(&scene, Vec3::new(10.0, 0.08, -2.0), listener),
        BandGain::UNITY
    );
}

/// Both flights can be climbed and descended without snapping onto a floor above one's head.
#[test]
fn stairs_join_correct_stories_and_floor_holes() {
    let (_, mut supports) = house_plan(10.0, true);
    for support in super::world::stair_supports() {
        let x = support.center.x;
        let y = support.height - 1.25;
        supports.push(support);
        let mut feet = Vec3::new(x, y, -4.1);
        for step in 0..=84 {
            feet.z = (step as f32).mul_add(0.05, -4.1);
            feet.y = support_height(feet, &supports);
        }
        assert!((feet.y - (y + 2.5)).abs() < 0.01, "climbed to {feet:?}");
        for step in 0..=84 {
            feet.z = (step as f32).mul_add(-0.05, 0.1);
            feet.y = support_height(feet, &supports);
        }
        assert!((feet.y - y).abs() < 0.01, "descended to {feet:?}");
    }
    assert_eq!(support_height(Vec3::new(10.0, 0.0, -2.0), &supports), 0.0);
}

/// Turning a third-person humanoid never rotates its overhead camera with it.
#[test]
fn overhead_camera_is_independent_of_mouse_heading() {
    let mut view = View::default();
    let original = camera_pose(Vec3::ZERO, &view);
    view.yaw = 1.7;
    assert_eq!(camera_pose(Vec3::ZERO, &view), original);
    view.first_person = true;
    assert!(
        camera_pose(Vec3::ZERO, &view)
            .forward()
            .dot(Quat::from_rotation_y(view.yaw) * Vec3::NEG_Z)
            > 0.999
    );
}

/// Actual rodio stereo output agrees with the character's colored ear cups at every heading.
#[test]
fn stereo_left_right_follows_listener_orientation() {
    let ears = SpatialListener::new(0.3);
    for yaw in [0.0, 0.7, 1.57, 3.14] {
        for distance in [0.2, 1.0, 4.0, 20.0] {
            for side in [-1.0, 1.0] {
                let listener =
                    Transform::from_xyz(0.0, EYE, 0.0).with_rotation(Quat::from_rotation_y(yaw));
                let source = listener.translation + *listener.right() * distance * side;
                let input = SamplesBuffer::new(
                    NonZeroU16::new(1).unwrap(),
                    NonZeroU32::new(48_000).unwrap(),
                    vec![0.5; 4],
                );
                let mut output = Spatial::new(
                    input,
                    (source * 0.22).into(),
                    (listener.transform_point(ears.left_ear_offset) * 0.22).into(),
                    (listener.transform_point(ears.right_ear_offset) * 0.22).into(),
                );
                let left = output.next().unwrap();
                let right = output.next().unwrap();
                assert!(
                    (right - left) * side > 0.0,
                    "yaw={yaw} side={side} L={left} R={right}"
                );
            }
        }
    }
}

/// Rotating a door frees its former opening and blocks the new visible leaf location.
#[test]
fn hinged_door_collisions_follow_the_leaf() {
    let door = Solid {
        center: Vec3::new(0.0, 1.05, 0.45),
        half: Vec3::new(0.055, 1.05, 0.45),
    };
    let closed = GlobalTransform::default();
    let open = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(1.5)));
    assert!(overlaps(Vec3::new(0.0, 0.0, 0.45), door, &closed));
    assert!(!overlaps(Vec3::new(0.0, 0.0, 0.65), door, &open));
    assert!(overlaps(Vec3::new(0.45, 0.0, 0.03), door, &open));
}

/// Isolated tests cannot be contaminated by rain, own steps, or another story's runner.
#[test]
fn isolated_mix_and_master_mute_survive_processing_bypass() {
    let mut mix = Mix::default();
    for (scenario, only) in [
        (1, Kind::Voice),
        (2, Kind::Upstairs),
        (3, Kind::Basement),
        (4, Kind::Stream),
    ] {
        mix.scenario = scenario;
        mix.rain = true;
        mix.steps = true;
        mix.processing = false;
        for kind in [
            Kind::Voice,
            Kind::Upstairs,
            Kind::Basement,
            Kind::Stream,
            Kind::Rain,
            Kind::OwnSteps,
        ] {
            assert_eq!(mix.enabled(kind), kind == only);
        }
        mix.muted = true;
        assert!(!mix.enabled(only));
        mix.muted = false;
    }
}

/// The high end of solid stairs blocks walking through them, without blocking the basement below.
#[test]
fn staircase_bulk_and_bridge_clearance_differ() {
    let stairs = WalkSurface {
        center: Vec2::new(13.0, -2.0),
        half: Vec2::new(0.65, 2.0),
        height: 1.25,
        slope: Vec2::new(0.0, 0.625),
        solid_from: Some(0.0),
    };
    assert!(stairs.blocks(Vec3::new(13.0, 0.0, -0.1)));
    assert!(!stairs.blocks(Vec3::new(13.0, 2.5, -0.1)));
    assert!(!stairs.blocks(Vec3::new(13.0, -2.5, -0.1)));
    assert!(!stairs.blocks(Vec3::new(13.0, 0.0, -3.9)));
    let bridge = WalkSurface {
        center: Vec2::new(0.0, super::world::BRIDGE_Z),
        half: Vec2::new(3.0, 0.85),
        height: 1.2,
        slope: Vec2::ZERO,
        solid_from: None,
    };
    assert!(!bridge.blocks(Vec3::new(0.0, -1.1, super::world::BRIDGE_Z)));
}

/// The actual movement system climbs and descends both stairs while testing all structural colliders.
#[test]
fn controller_traverses_stairs_with_house_collisions() {
    use super::{
        controller::{Player, walk},
        world::{Village, stair_supports},
    };
    use bevy::prelude::{App, ButtonInput, KeyCode, Real, Time, Update};
    use std::time::Duration;
    for stair in stair_supports() {
        let (modules, mut supports) = house_plan(10.0, true);
        supports.extend(stair_supports());
        let mut app = App::new();
        let mut clock = Time::<Real>::default();
        clock.advance_by(Duration::from_millis(40));
        app.insert_resource(clock)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<View>()
            .insert_resource(Village {
                supports,
                ..Default::default()
            })
            .add_systems(Update, walk);
        for module in modules.into_iter().filter(|m| m.blocks) {
            for solid in module.boxes {
                app.world_mut()
                    .spawn((solid, GlobalTransform::from(module.pose)));
            }
        }
        let base = stair.height - 1.25;
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(stair.center.x, base, -4.5)))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyS);
        for _ in 0..46 {
            app.update();
        }
        let top = app.world().get::<Transform>(player).unwrap().translation;
        assert!(
            (top.y - (base + 2.5)).abs() < 0.02 && top.z > 0.3,
            "did not climb: {top:?}"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        for _ in 0..46 {
            app.update();
        }
        let bottom = app.world().get::<Transform>(player).unwrap().translation;
        assert!(
            (bottom.y - base).abs() < 0.02 && bottom.z < -4.3,
            "did not descend: {bottom:?}"
        );
    }
}

/// Traces a room with the same floor and furniture materials used by the showcase.
fn treated_room(mode: usize) -> bevy_raytraced_audio::ReverbEstimate {
    use super::world::{box_faces, furniture_bounds};
    use bevy_raytraced_audio::{ListenerTrace3d, RayTraceSettings};
    let mut scene = AcousticScene3d::default();
    let mut add = |corners: [Vec3; 4], surface: Surface| {
        let [a, b, c, d] = corners.map(|p| Point3::try_new(p.x, p.y, p.z).unwrap());
        for vertices in [[a, b, c], [a, c, d]] {
            scene.add_triangle(Triangle3d::try_new(vertices, surface.material()).unwrap());
        }
    };
    for (face, points) in box_faces(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 2.5, 4.0))
        .into_iter()
        .enumerate()
    {
        add(
            points,
            if face == 2 {
                if mode == 0 {
                    Surface::Tile
                } else {
                    Surface::Carpet
                }
            } else {
                Surface::Wall
            },
        );
    }
    if mode == 2 {
        for (asset, position) in [
            ("loungeSofa", Vec3::new(-2.8, 0.0, -2.9)),
            ("bedDouble", Vec3::new(2.0, 0.0, 2.0)),
            ("bookcaseOpen", Vec3::new(2.8, 0.0, -3.6)),
        ] {
            let (min, max, surface) = furniture_bounds(asset);
            for points in box_faces(min + position, max + position) {
                add(points, surface);
            }
        }
    }
    let mut trace = ListenerTrace3d::default();
    scene.trace_listener(
        Listener3d::new(Point3::try_new(0.0, 1.5, 0.0).unwrap()),
        &[],
        RayTraceSettings::default()
            .try_with_ray_count(4096)
            .unwrap()
            .with_max_bounces(12),
        &mut trace,
    );
    trace.reverb()
}

/// Soft floors shorten treble decay; furnishing the same room absorbs additional energy.
#[test]
fn carpet_and_furniture_change_the_room_response() {
    let hard = treated_room(0);
    let carpet = treated_room(1);
    let furnished = treated_room(2);
    assert!(
        carpet.decay_high_s() < hard.decay_high_s() * 0.8,
        "{hard:?} {carpet:?}"
    );
    assert!(
        furnished.decay_high_s() < carpet.decay_high_s(),
        "{carpet:?} {furnished:?}"
    );
    assert!(
        furnished.wet_gain() < carpet.wet_gain(),
        "{carpet:?} {furnished:?}"
    );
    assert!(furnished.decay_high_s() < furnished.decay_low_s());
}
