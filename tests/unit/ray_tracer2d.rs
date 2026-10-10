//! Behavior checks for the listener ray tracer against small reference rooms.

use super::ListenerTrace2d;
use crate::{
    AcousticMaterial, AcousticScene2d, BandAbsorption, BandGain, Emitter2d, Listener2d, Point2,
    RayKind, RayTraceSettings, Segment2d,
};

/// Makes a finite point.
fn point(x: f32, y: f32) -> Point2 {
    Point2::try_new(x, y).expect("finite test point")
}

/// Adds one wall with the given material.
fn wall(
    scene: &mut AcousticScene2d,
    start: (f32, f32),
    end: (f32, f32),
    material: AcousticMaterial,
) {
    scene.add_segment(
        Segment2d::try_new(point(start.0, start.1), point(end.0, end.1), material)
            .expect("nondegenerate wall"),
    );
}

/// Lightly absorbing, opaque walls.
fn plaster() -> AcousticMaterial {
    AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.1, 0.15).expect("valid absorption"))
}

/// Builds a square room centred on the origin, optionally leaving a doorway in the east wall.
fn square_room(half: f32, doorway: Option<f32>) -> AcousticScene2d {
    let mut scene = AcousticScene2d::default();
    let material = plaster();
    wall(&mut scene, (-half, -half), (half, -half), material);
    wall(&mut scene, (-half, half), (half, half), material);
    wall(&mut scene, (-half, -half), (-half, half), material);
    match doorway {
        Some(gap) => {
            wall(&mut scene, (half, -half), (half, -gap), material);
            wall(&mut scene, (half, gap), (half, half), material);
        }
        None => wall(&mut scene, (half, -half), (half, half), material),
    }
    scene
}

/// Traces one listener and sources with default settings.
fn trace(scene: &AcousticScene2d, listener: (f32, f32), sources: &[(f32, f32)]) -> ListenerTrace2d {
    let sources: Vec<_> = sources
        .iter()
        .map(|(x, y)| Emitter2d::new(point(*x, *y)))
        .collect();
    let mut output = ListenerTrace2d::default();
    scene.trace_listener(
        Listener2d::new(point(listener.0, listener.1)),
        &sources,
        RayTraceSettings::default().with_recorded_rays(true),
        &mut output,
    );
    output
}

/// A sealed room hides an outside source completely and keeps all rays inside.
#[test]
fn sealed_room_fully_muffles_outside_source() {
    let scene = square_room(4.0, None);
    let output = trace(&scene, (0.0, 0.0), &[(10.0, 0.0)]);
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
    let reflective = square_room(4.0, None);
    let material = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0).unwrap());
    let mut absorbing = AcousticScene2d::default();
    for surface in reflective.segments() {
        absorbing
            .add_segment(Segment2d::try_new(surface.start(), surface.end(), material).unwrap());
    }
    let dry = trace(&absorbing, (0.0, 0.0), &[]).reverb();
    let wet = trace(&reflective, (0.0, 0.0), &[]).reverb();
    assert!(dry.wet_gain() < f32::EPSILON, "{dry:?}");
    assert!(wet.wet_gain() > 0.5, "{wet:?}");
}

/// With no geometry, every ray escapes and nothing reverberates.
#[test]
fn open_field_is_outdoor_and_dry() {
    let scene = AcousticScene2d::default();
    let output = trace(&scene, (0.0, 0.0), &[(3.0, 0.0)]);
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
    let mut scene = square_room(4.0, Some(1.0));
    // A second wall outside the doorway hides the source from a straight line.
    wall(&mut scene, (6.0, -3.0), (6.0, 3.0), plaster());
    let output = trace(&scene, (-2.0, 2.0), &[(8.0, 0.0)]);
    let source = output.sources()[0];
    assert!(!source.is_direct_visible());
    let sealed = trace(&square_room(4.0, None), (-2.0, 2.0), &[(8.0, 0.0)]).sources()[0];
    assert!(source.clarity() >= sealed.clarity());
    let around_corner =
        trace(&square_room(4.0, Some(1.0)), (-2.0, 2.0), &[(5.0, 3.0)]).sources()[0];
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
    let scene = square_room(4.0, Some(1.5));
    let output = trace(&scene, (-1.0, 0.0), &[]);
    let direction = output
        .ambient_direction()
        .expect("rays escape through the doorway");
    assert!(direction.x_m() > 0.8, "{direction:?}");
    let reverb = output.reverb();
    assert!(reverb.outdoor_fraction() > 0.0 && reverb.outdoor_fraction() < 1.0);
}

/// Recording emits primary, echo, occlusion, permeation, and escape segments.
#[test]
fn recording_contains_every_ray_kind() {
    let mut scene = square_room(4.0, Some(1.0));
    let thin = AcousticMaterial::default()
        .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
        .expect("valid material");
    wall(&mut scene, (0.0, -4.0), (0.0, 0.5), thin);
    let output = trace(&scene, (-2.0, -2.0), &[(2.0, -2.0)]);
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
}

/// Two transmitting walls muffle more than one.
#[test]
fn thicker_walls_transmit_less() {
    let leaky = AcousticMaterial::default()
        .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
        .expect("valid material");
    let mut thin = AcousticScene2d::default();
    wall(&mut thin, (0.0, -20.0), (0.0, 20.0), leaky);
    let mut thick = thin.clone();
    wall(&mut thick, (0.4, -20.0), (0.4, 20.0), leaky);
    let thin_response = trace(&thin, (-2.0, 0.0), &[(2.0, 0.0)]).sources()[0];
    let thick_response = trace(&thick, (-2.0, 0.0), &[(2.0, 0.0)]).sources()[0];
    assert!(thick_response.permeation().low() < thin_response.permeation().low());
    assert!(thick_response.filter().gain_lf() < thin_response.filter().gain_lf());
}

/// Cached results match a fresh trace after every independently mutable input changes.
#[test]
fn cache_invalidates_for_all_inputs_and_scene_clone_mutation() {
    let mut scene = square_room(3., None);
    let mut listener = Listener2d::new(point(0., 0.));
    let mut sources = vec![
        Emitter2d::new(point(1., 0.)),
        Emitter2d::new(point(-1., 0.5)),
    ];
    let mut settings = RayTraceSettings::default()
        .try_with_ray_count(8)
        .unwrap()
        .with_max_bounces(2)
        .with_recorded_rays(true);
    let mut cached = ListenerTrace2d::default();
    for step in 0..13 {
        match step {
            2 => *sources.first_mut().unwrap() = Emitter2d::new(point(1.5, 0.5)),
            3 => sources.reverse(),
            4 => sources.push(Emitter2d::new(point(0., 0.))),
            5 => {
                sources.pop();
            }
            6 => listener = Listener2d::new(point(-1., 0.5)),
            7 => settings = settings.with_seed(717),
            8 => settings = settings.with_recorded_rays(false),
            9 => {
                scene.clear();
            }
            10 => {
                scene = scene.clone();
                wall(&mut scene, (0., -2.), (0., 2.), plaster());
            }
            11 => {
                cached = cached.clone();
            }
            12 => {
                scene = AcousticScene2d::default();
                sources.clear();
            }
            _ => {}
        }
        scene.trace_listener(listener, &sources, settings, &mut cached);
        let mut fresh = ListenerTrace2d::default();
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
