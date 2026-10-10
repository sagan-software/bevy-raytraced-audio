//! Behavior checks for the 3D listener ray tracer against small reference rooms.

use super::{FibonacciSphere, ListenerTrace3d, tangent_basis};
use crate::math3d::Vector3;
use crate::ray_trace::{EchoStatistics, TraceRng};
use crate::{
    AcousticMaterial, AcousticScene3d, BandAbsorption, BandGain, Emitter3d, Listener3d, Point3,
    RayKind, RayTraceSettings, Triangle3d,
};

/// Makes a finite point.
fn point(x: f32, y: f32, z: f32) -> Point3 {
    Point3::try_new(x, y, z).expect("finite test point")
}

/// Adds one planar quad as two triangles sharing the `a`–`c` diagonal.
fn quad(scene: &mut AcousticScene3d, corners: [(f32, f32, f32); 4], material: AcousticMaterial) {
    let [a, b, c, d] = corners.map(|(x, y, z)| point(x, y, z));
    for vertices in [[a, b, c], [a, c, d]] {
        scene.add_triangle(Triangle3d::try_new(vertices, material).expect("nondegenerate quad"));
    }
}

/// Lightly absorbing, opaque walls.
fn plaster() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.1, 0.15).expect("valid absorption"))
}

/// Half-transmissive walls that pass bass more readily than treble.
fn leaky() -> AcousticMaterial {
    AcousticMaterial::default()
        .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
        .expect("valid material")
}

/// Builds an axis-aligned box room centred on the origin with `z` up.
///
/// A doorway of half-width `gap` leaves a hole in the east wall from the floor to `z = gap`,
/// framed by two side quads and a lintel above it.
fn box_room((hx, hy, hz): (f32, f32, f32), doorway: Option<f32>) -> AcousticScene3d {
    let mut scene = AcousticScene3d::default();
    let material = plaster();
    quad(
        &mut scene,
        [
            (-hx, -hy, -hz),
            (hx, -hy, -hz),
            (hx, hy, -hz),
            (-hx, hy, -hz),
        ],
        material,
    );
    quad(
        &mut scene,
        [(-hx, -hy, hz), (hx, -hy, hz), (hx, hy, hz), (-hx, hy, hz)],
        material,
    );
    quad(
        &mut scene,
        [
            (-hx, -hy, -hz),
            (hx, -hy, -hz),
            (hx, -hy, hz),
            (-hx, -hy, hz),
        ],
        material,
    );
    quad(
        &mut scene,
        [(-hx, hy, -hz), (hx, hy, -hz), (hx, hy, hz), (-hx, hy, hz)],
        material,
    );
    quad(
        &mut scene,
        [
            (-hx, -hy, -hz),
            (-hx, hy, -hz),
            (-hx, hy, hz),
            (-hx, -hy, hz),
        ],
        material,
    );
    match doorway {
        Some(gap) => {
            quad(
                &mut scene,
                [
                    (hx, -hy, -hz),
                    (hx, -gap, -hz),
                    (hx, -gap, hz),
                    (hx, -hy, hz),
                ],
                material,
            );
            quad(
                &mut scene,
                [(hx, gap, -hz), (hx, hy, -hz), (hx, hy, hz), (hx, gap, hz)],
                material,
            );
            quad(
                &mut scene,
                [
                    (hx, -gap, gap),
                    (hx, gap, gap),
                    (hx, gap, hz),
                    (hx, -gap, hz),
                ],
                material,
            );
        }
        None => {
            quad(
                &mut scene,
                [(hx, -hy, -hz), (hx, hy, -hz), (hx, hy, hz), (hx, -hy, hz)],
                material,
            );
        }
    }
    scene
}

/// Traces one listener and sources with default settings and recording enabled.
fn trace(
    scene: &AcousticScene3d,
    listener: (f32, f32, f32),
    sources: &[(f32, f32, f32)],
) -> ListenerTrace3d {
    let sources: Vec<_> = sources
        .iter()
        .map(|(x, y, z)| Emitter3d::new(point(*x, *y, *z)))
        .collect();
    let mut output = ListenerTrace3d::default();
    scene.trace_listener(
        Listener3d::new(point(listener.0, listener.1, listener.2)),
        &sources,
        RayTraceSettings::default().with_recorded_rays(true),
        &mut output,
    );
    output
}

/// Spiral directions are unit length and balanced around the sphere.
#[test]
fn fibonacci_directions_cover_the_sphere() {
    let mut rng = TraceRng::new(3);
    let spiral = FibonacciSphere::new(256, &mut rng);
    let mut sum = Vector3::default();
    let mut second_moment = Vector3::default();
    for index in 0..256 {
        let direction = spiral.direction(index);
        assert!((direction.length() - 1.0).abs() < 1.0e-12);
        sum = sum.add(direction);
        second_moment = second_moment.add(Vector3 {
            x: direction.x.powi(2),
            y: direction.y.powi(2),
            z: direction.z.powi(2),
        });
    }
    assert!(sum.length() / 256.0 < 0.01, "{sum:?}");
    // A horizontal circle can have zero mean, but cannot pass this isotropy check.
    for moment in [second_moment.x, second_moment.y, second_moment.z] {
        assert!((moment / 256.0 - 1.0 / 3.0).abs() < 0.01);
    }
}

/// Tangent bases are orthonormal, including for normals pointing straight down.
#[test]
fn tangent_basis_is_orthonormal() {
    for normal in [
        Vector3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Vector3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        Vector3 {
            x: 0.6,
            y: 0.0,
            z: 0.8,
        },
        Vector3 {
            x: 0.0,
            y: -0.6,
            z: -0.8,
        },
    ] {
        let (first, second) = tangent_basis(normal);
        assert!((first.length() - 1.0).abs() < 1.0e-12);
        assert!((second.length() - 1.0).abs() < 1.0e-12);
        assert!(first.dot(normal).abs() < 1.0e-12);
        assert!(second.dot(normal).abs() < 1.0e-12);
        assert!(first.dot(second).abs() < 1.0e-12);
    }
}

/// A sealed room hides an outside source completely and keeps all rays inside.
#[test]
fn sealed_room_fully_muffles_outside_source() {
    let scene = box_room((4.0, 4.0, 2.0), None);
    let output = trace(&scene, (0.0, 0.0, 0.0), &[(10.0, 0.0, 0.0)]);
    let source = output.sources()[0];
    assert!(!source.is_direct_visible());
    assert!(source.discovered_fraction() < f32::EPSILON);
    assert!(source.filter().gain_hf() < 1.0e-6);
    let reverb = output.reverb();
    assert!(reverb.outdoor_fraction() < f32::EPSILON);
    assert!(reverb.return_fraction() > 0.5, "{reverb:?}");
    assert!(reverb.decay_time_s() > 0.2, "{reverb:?}");
    assert!(reverb.wet_gain() > 0.5, "{reverb:?}");
    assert!(output.ambient_direction().is_none());
}

/// Fully absorbing room surfaces leave no energy for audible reverberation.
#[test]
fn absorbing_room_has_no_reverb_send() {
    let reflective = box_room((4.0, 4.0, 2.0), None);
    let material = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0).unwrap());
    let mut absorbing = AcousticScene3d::default();
    for surface in reflective.triangles() {
        absorbing.add_triangle(Triangle3d::try_new(surface.vertices(), material).unwrap());
    }
    let dry = trace(&absorbing, (0.0, 0.0, 0.0), &[]).reverb();
    let wet = trace(&reflective, (0.0, 0.0, 0.0), &[]).reverb();
    assert!(dry.wet_gain() < f32::EPSILON, "{dry:?}");
    assert!(wet.wet_gain() > 0.5, "{wet:?}");
}

/// With no geometry, every ray escapes and nothing reverberates.
#[test]
fn open_field_is_outdoor_and_dry() {
    let scene = AcousticScene3d::default();
    let output = trace(&scene, (0.0, 0.0, 0.0), &[(3.0, 0.0, 0.0)]);
    assert!(output.sources()[0].is_direct_visible());
    assert!((output.sources()[0].clarity() - 1.0).abs() < f32::EPSILON);
    let reverb = output.reverb();
    assert!((reverb.outdoor_fraction() - 1.0).abs() < f32::EPSILON);
    assert!(reverb.wet_gain() < f32::EPSILON);
    assert!(output.ambient_focus() < 0.05, "uniform escapes cancel out");
}

/// A source in the next room through a doorway is partly discovered by bounces.
#[test]
fn doorway_partially_muffles_hidden_source() {
    let mut scene = box_room((4.0, 4.0, 2.0), Some(1.0));
    // A second wall outside the doorway hides the source from a straight line.
    quad(
        &mut scene,
        [
            (6.0, -3.0, -2.0),
            (6.0, 3.0, -2.0),
            (6.0, 3.0, 2.0),
            (6.0, -3.0, 2.0),
        ],
        plaster(),
    );
    let output = trace(&scene, (-2.0, 2.0, 0.0), &[(8.0, 0.0, 0.0)]);
    let source = output.sources()[0];
    assert!(!source.is_direct_visible());
    let sealed = trace(
        &box_room((4.0, 4.0, 2.0), None),
        (-2.0, 2.0, 0.0),
        &[(8.0, 0.0, 0.0)],
    )
    .sources()[0];
    assert!(source.clarity() >= sealed.clarity());
    let around_corner = trace(
        &box_room((4.0, 4.0, 2.0), Some(1.0)),
        (-2.0, 2.0, 0.0),
        &[(5.0, 3.0, 0.0)],
    )
    .sources()[0];
    assert!(!around_corner.is_direct_visible());
    assert!(
        around_corner.discovered_fraction() > 0.0,
        "{around_corner:?}"
    );
    assert!(around_corner.clarity() > 0.0 && around_corner.clarity() < 1.0);
    assert!(around_corner.filter().gain_hf() < around_corner.filter().gain_lf());
}

/// Escaping rays through an east doorway point the ambience east.
#[test]
fn ambience_arrives_from_the_doorway() {
    let scene = box_room((4.0, 4.0, 2.0), Some(1.5));
    let output = trace(&scene, (-1.0, 0.0, 0.0), &[]);
    let direction = output
        .ambient_direction()
        .expect("rays escape through the doorway");
    let length = direction
        .x_m()
        .hypot(direction.y_m())
        .hypot(direction.z_m());
    assert!((length - 1.0).abs() < 1.0e-9, "{direction:?}");
    assert!(direction.x_m() > 0.8, "{direction:?}");
    let reverb = output.reverb();
    assert!(reverb.outdoor_fraction() > 0.0 && reverb.outdoor_fraction() < 1.0);
}

/// Recording emits primary, echo, occlusion, permeation, and escape segments.
#[test]
fn recording_contains_every_ray_kind() {
    let mut scene = box_room((4.0, 4.0, 2.0), Some(1.0));
    quad(
        &mut scene,
        [
            (0.0, -4.0, -2.0),
            (0.0, 0.5, -2.0),
            (0.0, 0.5, 2.0),
            (0.0, -4.0, 2.0),
        ],
        leaky(),
    );
    let output = trace(&scene, (-2.0, -2.0, 0.0), &[(2.0, -2.0, 0.0)]);
    for kind in [
        RayKind::Primary,
        RayKind::Echo,
        RayKind::Occlusion,
        RayKind::Permeation,
        RayKind::Escaped,
        RayKind::Ambient,
    ] {
        assert!(
            output
                .segments()
                .iter()
                .any(|segment| segment.kind() == kind),
            "missing {kind:?}"
        );
    }
    let primary_distances_increase = output
        .segments()
        .windows(2)
        .filter(|pair| pair[0].kind() == RayKind::Primary && pair[1].kind() == RayKind::Primary)
        .all(|pair| {
            pair[1].start_distance_m() == 0.0
                || pair[1].start_distance_m() >= pair[0].start_distance_m()
        });
    assert!(primary_distances_increase);
    assert!(
        output
            .segments()
            .iter()
            .all(|segment| segment.length_m() >= 0.0)
    );
}

/// Two transmitting walls muffle more than one.
#[test]
fn thicker_walls_transmit_less() {
    let mut thin = AcousticScene3d::default();
    quad(
        &mut thin,
        [
            (0.0, -20.0, -20.0),
            (0.0, 20.0, -20.0),
            (0.0, 20.0, 20.0),
            (0.0, -20.0, 20.0),
        ],
        leaky(),
    );
    let mut thick = thin.clone();
    quad(
        &mut thick,
        [
            (0.4, -20.0, -20.0),
            (0.4, 20.0, -20.0),
            (0.4, 20.0, 20.0),
            (0.4, -20.0, 20.0),
        ],
        leaky(),
    );
    let thin_response = trace(&thin, (-2.0, 0.0, 0.0), &[(2.0, 0.0, 0.0)]).sources()[0];
    let thick_response = trace(&thick, (-2.0, 0.0, 0.0), &[(2.0, 0.0, 0.0)]).sources()[0];
    assert!(thick_response.permeation().low() < thin_response.permeation().low());
    assert!(thick_response.filter().gain_lf() < thin_response.filter().gain_lf());
}
/// An isolated outdoor wall gives an early reflection without a sealed-room tail.
#[test]
fn outdoor_wall_retains_early_echoes() {
    let mut scene = AcousticScene3d::default();
    quad(
        &mut scene,
        [
            (-100.0, -100.0, -3.0),
            (100.0, -100.0, -3.0),
            (100.0, 100.0, -3.0),
            (-100.0, 100.0, -3.0),
        ],
        plaster(),
    );
    let reverb = trace(&scene, (0.0, 0.0, 0.0), &[]).reverb();
    assert!(reverb.outdoor_fraction() > 0.99, "{reverb:?}");
    assert!(reverb.wet_gain() < 1.0e-6, "{reverb:?}");
    assert!(reverb.early_gain() > 0.1, "{reverb:?}");
    assert!(reverb.reflections_delay_s() > 6.0 / 343.0, "{reverb:?}");
}

/// Cached results match a fresh trace after every independently mutable input changes.
#[test]
fn cache_invalidates_for_all_inputs_and_scene_clone_mutation() {
    let mut scene = box_room((3., 3., 3.), None);
    let mut listener = Listener3d::new(point(0., 0., 0.));
    let mut sources = vec![
        Emitter3d::new(point(1., 0., 0.)),
        Emitter3d::new(point(-1., 0.5, 0.)),
    ];
    let mut settings = RayTraceSettings::default()
        .try_with_ray_count(8)
        .unwrap()
        .with_max_bounces(2)
        .with_recorded_rays(true);
    let mut cached = ListenerTrace3d::default();
    for step in 0..13 {
        match step {
            2 => *sources.first_mut().unwrap() = Emitter3d::new(point(1.5, 0.5, 0.)),
            3 => sources.reverse(),
            4 => sources.push(Emitter3d::new(point(0., 0., 0.))),
            5 => {
                sources.pop();
            }
            6 => listener = Listener3d::new(point(-1., 0.5, 0.)),
            7 => settings = settings.with_seed(717),
            8 => settings = settings.with_recorded_rays(false),
            9 => {
                scene.clear();
            }
            10 => {
                scene = scene.clone();
                quad(
                    &mut scene,
                    [(0., -2., -2.), (0., 2., -2.), (0., 2., 2.), (0., -2., 2.)],
                    plaster(),
                );
            }
            11 => {
                cached = cached.clone();
            }
            12 => {
                scene = AcousticScene3d::default();
                sources.clear();
            }
            _ => {}
        }
        scene.trace_listener(listener, &sources, settings, &mut cached);
        let mut fresh = ListenerTrace3d::default();
        scene.trace_listener(listener, &sources, settings, &mut fresh);
        assert_eq!(cached.sources(), fresh.sources(), "step {step}");
        assert_eq!(cached.reverb(), fresh.reverb(), "step {step}");
        assert_eq!(
            cached.ambient_direction(),
            fresh.ambient_direction(),
            "step {step}"
        );
        assert_eq!(cached.ambient_focus(), fresh.ambient_focus(), "step {step}");
        assert_eq!(cached.segments(), fresh.segments(), "step {step}");
    }
}

/// Degenerate arrival paths do not invent an ambience direction or produce NaN scattering.
#[test]
fn escape_and_scatter_boundaries_are_finite() {
    let zero = Vector3::default();
    let normal = Vector3 {
        x: 0.,
        y: 0.,
        z: 1.,
    };
    let mut statistics = EchoStatistics::default();
    let mut output = ListenerTrace3d::default();
    for (bounce, last_echo) in [(1, None), (0, Some(zero))] {
        assert_eq!(
            AcousticScene3d::escape(
                (zero, zero, normal),
                (bounce, 0., 1.),
                last_echo,
                RayTraceSettings::default().with_recorded_rays(true),
                &mut statistics,
                &mut output
            ),
            None
        );
    }
    let scene = AcousticScene3d::default();
    assert!(!scene.segment_is_blocked(zero, zero, None));
    let mut rng = TraceRng::new(7);
    assert_eq!(super::scatter(normal, normal, 0., &mut rng), normal);
    let diffuse = super::scatter(normal, normal, 1., &mut TraceRng::new(7));
    assert_eq!(
        super::scatter(diffuse.scale(-1.), normal, 0.5, &mut TraceRng::new(7)),
        normal
    );
}
