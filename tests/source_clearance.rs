//! Public contract for the optional per-emitter source clearance radius.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene2d, AcousticScene3d, BandGain, Emitter2d, Emitter3d,
    GeometryError, Listener2d, Listener3d, ListenerTrace2d, ListenerTrace3d, Point2, Point3,
    RayTraceSettings, Segment2d, Triangle3d,
};

/// Builds a finite 3D point from literal test coordinates.
const fn p3(x: f32, y: f32, z: f32) -> Result<Point3, GeometryError> {
    Point3::try_new(x, y, z)
}

/// Builds a finite 2D point from literal test coordinates.
const fn p2(x: f32, y: f32) -> Result<Point2, GeometryError> {
    Point2::try_new(x, y)
}

/// Adds two triangles covering the quad `a b c d` with a fully blocking default material.
fn add_quad(scene: &mut AcousticScene3d, corners: [Point3; 4]) -> Result<(), GeometryError> {
    let [a, b, c, d] = corners;
    scene.add_triangle(Triangle3d::try_new([a, b, c], AcousticMaterial::default())?);
    scene.add_triangle(Triangle3d::try_new([a, c, d], AcousticMaterial::default())?);
    Ok(())
}

/// Adds a closed axis-aligned cube of half-size `half` centred on the origin.
fn add_cube(scene: &mut AcousticScene3d, half: f32) -> Result<(), GeometryError> {
    let h = half;
    for sign in [-h, h] {
        add_quad(
            scene,
            [
                p3(sign, -h, -h)?,
                p3(sign, h, -h)?,
                p3(sign, h, h)?,
                p3(sign, -h, h)?,
            ],
        )?;
        add_quad(
            scene,
            [
                p3(-h, sign, -h)?,
                p3(h, sign, -h)?,
                p3(h, sign, h)?,
                p3(-h, sign, h)?,
            ],
        )?;
        add_quad(
            scene,
            [
                p3(-h, -h, sign)?,
                p3(h, -h, sign)?,
                p3(h, h, sign)?,
                p3(-h, h, sign)?,
            ],
        )?;
    }
    Ok(())
}

/// Adds a closed axis-aligned square of half-size `half` centred on the origin.
fn add_square(scene: &mut AcousticScene2d, half: f32) -> Result<(), GeometryError> {
    let h = half;
    let corners = [p2(-h, -h)?, p2(h, -h)?, p2(h, h)?, p2(-h, h)?];
    for (start, end) in corners.iter().zip(corners.iter().cycle().skip(1)) {
        scene.add_segment(Segment2d::try_new(
            *start,
            *end,
            AcousticMaterial::default(),
        )?);
    }
    Ok(())
}

#[test]
fn clearance_rejects_non_finite_and_negative_values() -> Result<(), GeometryError> {
    let emitter_3d = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let emitter_2d = Emitter2d::new(p2(0.0, 0.0)?);
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            emitter_3d.try_with_clearance(value),
            Err(GeometryError::NonFiniteValue)
        );
        assert_eq!(
            emitter_2d.try_with_clearance(value),
            Err(GeometryError::NonFiniteValue)
        );
    }
    for value in [-0.1, -f32::MIN_POSITIVE] {
        assert_eq!(
            emitter_3d.try_with_clearance(value),
            Err(GeometryError::CoefficientOutOfRange)
        );
        assert_eq!(
            emitter_2d.try_with_clearance(value),
            Err(GeometryError::CoefficientOutOfRange)
        );
    }
    assert_eq!(emitter_3d.try_with_clearance(1.5)?.clearance_m(), 1.5);
    assert_eq!(emitter_2d.try_with_clearance(1.5)?.clearance_m(), 1.5);
    Ok(())
}

#[test]
fn zero_clearance_is_the_default_and_preserves_results() -> Result<(), GeometryError> {
    let mut scene_3d = AcousticScene3d::default();
    add_cube(&mut scene_3d, 0.5)?;
    let emitter_3d = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    assert_eq!(emitter_3d.clearance_m(), 0.0);
    // Negative zero normalizes to the same default emitter.
    for zero in [0.0, -0.0] {
        let explicit = emitter_3d.try_with_clearance(zero)?;
        assert_eq!(explicit, emitter_3d);
        assert!(explicit.clearance_m().is_sign_positive());
    }
    let listener_3d = Listener3d::new(p3(3.0, 0.2, 0.1)?);
    assert_eq!(
        scene_3d.trace(emitter_3d.try_with_clearance(0.0)?, listener_3d),
        scene_3d.trace(emitter_3d, listener_3d)
    );

    let mut scene_2d = AcousticScene2d::default();
    add_square(&mut scene_2d, 0.5)?;
    let emitter_2d = Emitter2d::new(p2(0.0, 0.0)?);
    assert_eq!(emitter_2d.clearance_m(), 0.0);
    assert_eq!(emitter_2d.try_with_clearance(-0.0)?, emitter_2d);
    let listener_2d = Listener2d::new(p2(3.0, 0.2)?);
    assert_eq!(
        scene_2d.trace(emitter_2d.try_with_clearance(0.0)?, listener_2d),
        scene_2d.trace(emitter_2d, listener_2d)
    );
    Ok(())
}

#[test]
fn clearance_larger_than_an_enclosing_box_unoccludes_the_source() -> Result<(), GeometryError> {
    let mut scene = AcousticScene3d::default();
    add_cube(&mut scene, 0.5)?;
    let emitter = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let listener = Listener3d::new(p3(4.0, 0.3, -0.2)?);

    let enclosed = scene.trace(emitter, listener);
    assert!(enclosed.direct.is_occluded());
    assert_eq!(enclosed.direct.gain(), BandGain::ZERO);

    // The cube's corners lie 0.87 m from the centre, so a 1 m clearance covers every face.
    let cleared = scene.trace(emitter.try_with_clearance(1.0)?, listener);
    assert!(!cleared.direct.is_occluded());
    assert_eq!(cleared.direct.gain(), BandGain::UNITY);
    assert_eq!(cleared.direct.distance_m(), enclosed.direct.distance_m());
    // The cube's inner faces reflect only inside the clearance.
    assert_eq!(
        scene
            .reflection_paths(emitter.try_with_clearance(1.0)?, listener)
            .count(),
        0
    );

    // A clearance smaller than the cube keeps the source enclosed.
    assert!(
        scene
            .trace(emitter.try_with_clearance(0.4)?, listener)
            .direct
            .is_occluded()
    );
    // A listener inside the clearance hears the source directly.
    let near = Listener3d::new(p3(0.2, 0.0, 0.0)?);
    assert!(
        !scene
            .trace(emitter.try_with_clearance(1.0)?, near)
            .direct
            .is_occluded()
    );
    Ok(())
}

#[test]
fn walls_beyond_the_clearance_still_occlude() -> Result<(), GeometryError> {
    let mut scene = AcousticScene3d::default();
    add_cube(&mut scene, 0.5)?;
    add_quad(
        &mut scene,
        [
            p3(2.0, -5.0, -5.0)?,
            p3(2.0, 5.0, -5.0)?,
            p3(2.0, 5.0, 5.0)?,
            p3(2.0, -5.0, 5.0)?,
        ],
    )?;
    let emitter = Emitter3d::new(p3(0.0, 0.0, 0.0)?).try_with_clearance(1.0)?;
    let response = scene.trace(emitter, Listener3d::new(p3(4.0, 0.0, 0.0)?));
    assert!(response.direct.is_occluded());
    assert_eq!(response.direct.gain(), BandGain::ZERO);
    Ok(())
}

#[test]
fn reflections_inside_the_clearance_are_dropped() -> Result<(), GeometryError> {
    let mut scene = AcousticScene3d::default();
    // Index 0 and 1: a small floor patch just below the source.
    add_quad(
        &mut scene,
        [
            p3(-0.3, -0.1, -0.3)?,
            p3(0.3, -0.1, -0.3)?,
            p3(0.3, -0.1, 0.3)?,
            p3(-0.3, -0.1, 0.3)?,
        ],
    )?;
    // Index 2 and 3: a large wall three metres behind the source.
    add_quad(
        &mut scene,
        [
            p3(-3.0, -5.0, -5.0)?,
            p3(-3.0, 5.0, -5.0)?,
            p3(-3.0, 5.0, 5.0)?,
            p3(-3.0, -5.0, 5.0)?,
        ],
    )?;
    let emitter = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let listener = Listener3d::new(p3(2.0, 2.0, 0.0)?);

    let all: Vec<_> = scene.reflection_paths(emitter, listener).collect();
    let near = all
        .iter()
        .filter(|path| path.surface_index().index() < 2)
        .count();
    let far: Vec<_> = all
        .iter()
        .copied()
        .filter(|path| path.surface_index().index() >= 2)
        .collect();
    assert_eq!(near, 1);
    assert_eq!(far.len(), 1);

    let cleared: Vec<_> = scene
        .reflection_paths(emitter.try_with_clearance(0.5)?, listener)
        .collect();
    assert_eq!(cleared, far);
    let mut captured = Vec::new();
    let response = scene.trace_with_reflection_paths(
        emitter.try_with_clearance(0.5)?,
        listener,
        &mut captured,
    );
    assert_eq!(captured, far);
    assert!(
        response.reflected_energy.mid() < scene.trace(emitter, listener).reflected_energy.mid()
    );
    Ok(())
}

#[test]
fn listener_trace_honours_clearance_and_invalidates_its_cache() -> Result<(), GeometryError> {
    let mut scene = AcousticScene3d::default();
    add_cube(&mut scene, 0.5)?;
    add_quad(
        &mut scene,
        [
            p3(-6.0, -2.0, -6.0)?,
            p3(6.0, -2.0, -6.0)?,
            p3(6.0, -2.0, 6.0)?,
            p3(-6.0, -2.0, 6.0)?,
        ],
    )?;
    let listener = Listener3d::new(p3(4.0, 0.0, 0.3)?);
    let settings = RayTraceSettings::default()
        .try_with_ray_count(64)?
        .with_permeation_ray_count(8)
        .with_result_reuse(true);
    let emitter = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let mut trace = ListenerTrace3d::default();

    scene.trace_listener(listener, &[emitter], settings, &mut trace);
    assert!(!trace.sources()[0].is_direct_visible());
    assert_eq!(trace.sources()[0].permeation(), BandGain::ZERO);

    scene.trace_listener(
        listener,
        &[emitter.try_with_clearance(1.0)?],
        settings,
        &mut trace,
    );
    assert_eq!(trace.cache_statistics().source_changes, 1);
    let cleared = trace.sources()[0];
    assert!(cleared.is_direct_visible());
    assert!(cleared.discovered_fraction() > 0.0);
    Ok(())
}

#[test]
fn listener_trace_discovers_sources_only_past_the_clearance() -> Result<(), GeometryError> {
    let mut scene = AcousticScene3d::default();
    add_cube(&mut scene, 0.5)?;
    // A floor gives listener rays bounce points that can see the source over the cube.
    add_quad(
        &mut scene,
        [
            p3(-6.0, -2.0, -6.0)?,
            p3(6.0, -2.0, -6.0)?,
            p3(6.0, -2.0, 6.0)?,
            p3(-6.0, -2.0, 6.0)?,
        ],
    )?;
    // A blocking wall between listener and source keeps the source hidden and permeating.
    add_quad(
        &mut scene,
        [
            p3(2.0, -1.0, -1.0)?,
            p3(2.0, 1.0, -1.0)?,
            p3(2.0, 1.0, 1.0)?,
            p3(2.0, -1.0, 1.0)?,
        ],
    )?;
    let listener = Listener3d::new(p3(4.0, 0.0, 0.0)?);
    let settings = RayTraceSettings::default()
        .try_with_ray_count(256)?
        .with_permeation_ray_count(32);
    let emitter = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let mut trace = ListenerTrace3d::default();

    scene.trace_listener(listener, &[emitter], settings, &mut trace);
    let enclosed = trace.sources()[0];
    scene.trace_listener(
        listener,
        &[emitter.try_with_clearance(1.0)?],
        settings,
        &mut trace,
    );
    let cleared = trace.sources()[0];
    assert!(!enclosed.is_direct_visible());
    assert!(!cleared.is_direct_visible());
    assert_eq!(enclosed.discovered_fraction(), 0.0);
    assert!(cleared.discovered_fraction() > 0.0);
    Ok(())
}

#[test]
fn two_dimensional_clearance_matches_the_three_dimensional_contract() -> Result<(), GeometryError> {
    let mut scene = AcousticScene2d::default();
    add_square(&mut scene, 0.5)?;
    let emitter = Emitter2d::new(p2(0.0, 0.0)?);
    let listener = Listener2d::new(p2(4.0, 0.3)?);

    assert!(scene.trace(emitter, listener).direct.is_occluded());
    let cleared = scene.trace(emitter.try_with_clearance(1.0)?, listener);
    assert!(!cleared.direct.is_occluded());
    assert_eq!(cleared.direct.gain(), BandGain::UNITY);
    assert!(
        scene
            .trace(emitter.try_with_clearance(0.4)?, listener)
            .direct
            .is_occluded()
    );
    assert!(
        !scene
            .trace(
                emitter.try_with_clearance(1.0)?,
                Listener2d::new(p2(0.2, 0.0)?)
            )
            .direct
            .is_occluded()
    );

    // A wall beyond the clearance still blocks.
    scene.add_segment(Segment2d::try_new(
        p2(2.0, -5.0)?,
        p2(2.0, 5.0)?,
        AcousticMaterial::default(),
    )?);
    assert!(
        scene
            .trace(emitter.try_with_clearance(1.0)?, listener)
            .direct
            .is_occluded()
    );
    Ok(())
}

#[test]
fn two_dimensional_reflections_inside_the_clearance_are_dropped() -> Result<(), GeometryError> {
    let mut scene = AcousticScene2d::default();
    scene.add_segment(Segment2d::try_new(
        p2(-0.3, -0.1)?,
        p2(0.3, -0.1)?,
        AcousticMaterial::default(),
    )?);
    scene.add_segment(Segment2d::try_new(
        p2(-3.0, -5.0)?,
        p2(-3.0, 5.0)?,
        AcousticMaterial::default(),
    )?);
    let emitter = Emitter2d::new(p2(0.0, 0.0)?);
    let listener = Listener2d::new(p2(2.0, 2.0)?);

    let all: Vec<_> = scene.reflection_paths(emitter, listener).collect();
    assert_eq!(all.len(), 2);
    let cleared: Vec<_> = scene
        .reflection_paths(emitter.try_with_clearance(0.5)?, listener)
        .collect();
    assert_eq!(cleared.len(), 1);
    assert_eq!(cleared[0], all[1]);
    assert_eq!(cleared[0].surface_index().index(), 1);
    Ok(())
}

#[test]
fn two_dimensional_listener_trace_honours_clearance() -> Result<(), GeometryError> {
    let mut scene = AcousticScene2d::default();
    add_square(&mut scene, 0.5)?;
    scene.add_segment(Segment2d::try_new(
        p2(2.0, -1.0)?,
        p2(2.0, 1.0)?,
        AcousticMaterial::default(),
    )?);
    scene.add_segment(Segment2d::try_new(
        p2(-6.0, -2.0)?,
        p2(6.0, -2.0)?,
        AcousticMaterial::default(),
    )?);
    let listener = Listener2d::new(p2(4.0, 0.0)?);
    let settings = RayTraceSettings::default()
        .try_with_ray_count(256)?
        .with_permeation_ray_count(32)
        .with_result_reuse(true);
    let emitter = Emitter2d::new(p2(0.0, 0.0)?);
    let mut trace = ListenerTrace2d::default();

    scene.trace_listener(listener, &[emitter], settings, &mut trace);
    let enclosed = trace.sources()[0];
    scene.trace_listener(
        listener,
        &[emitter.try_with_clearance(1.0)?],
        settings,
        &mut trace,
    );
    assert_eq!(trace.cache_statistics().source_changes, 1);
    let cleared = trace.sources()[0];
    assert_eq!(enclosed.discovered_fraction(), 0.0);
    assert!(cleared.discovered_fraction() > 0.0);

    // Without the blocking wall the cleared source is directly visible.
    let mut open = AcousticScene2d::default();
    add_square(&mut open, 0.5)?;
    open.trace_listener(listener, &[emitter], settings, &mut trace);
    assert!(!trace.sources()[0].is_direct_visible());
    open.trace_listener(
        listener,
        &[emitter.try_with_clearance(1.0)?],
        settings,
        &mut trace,
    );
    assert!(trace.sources()[0].is_direct_visible());
    Ok(())
}

#[test]
fn listener_rays_do_not_discover_sources_from_hits_inside_the_clearance()
-> Result<(), GeometryError> {
    // The only surface is a 1 m floor patch just below the source; its corners lie 0.71 m away.
    let mut scene_3d = AcousticScene3d::default();
    add_quad(
        &mut scene_3d,
        [
            p3(-0.5, -0.1, -0.5)?,
            p3(0.5, -0.1, -0.5)?,
            p3(0.5, -0.1, 0.5)?,
            p3(-0.5, -0.1, 0.5)?,
        ],
    )?;
    let settings = RayTraceSettings::default().try_with_ray_count(1024)?;
    let emitter_3d = Emitter3d::new(p3(0.0, 0.0, 0.0)?);
    let listener_3d = Listener3d::new(p3(0.0, 1.5, 0.0)?);
    let mut trace_3d = ListenerTrace3d::default();

    scene_3d.trace_listener(listener_3d, &[emitter_3d], settings, &mut trace_3d);
    assert!(trace_3d.sources()[0].discovered_fraction() > 0.0);
    scene_3d.trace_listener(
        listener_3d,
        &[emitter_3d.try_with_clearance(0.75)?],
        settings,
        &mut trace_3d,
    );
    assert!(trace_3d.sources()[0].is_direct_visible());
    assert_eq!(trace_3d.sources()[0].discovered_fraction(), 0.0);

    // The 2D floor segment ends 0.51 m from the source.
    let mut scene_2d = AcousticScene2d::default();
    scene_2d.add_segment(Segment2d::try_new(
        p2(-0.5, -0.1)?,
        p2(0.5, -0.1)?,
        AcousticMaterial::default(),
    )?);
    let emitter_2d = Emitter2d::new(p2(0.0, 0.0)?);
    let listener_2d = Listener2d::new(p2(0.0, 1.5)?);
    let mut trace_2d = ListenerTrace2d::default();

    scene_2d.trace_listener(listener_2d, &[emitter_2d], settings, &mut trace_2d);
    assert!(trace_2d.sources()[0].discovered_fraction() > 0.0);
    scene_2d.trace_listener(
        listener_2d,
        &[emitter_2d.try_with_clearance(0.75)?],
        settings,
        &mut trace_2d,
    );
    assert!(trace_2d.sources()[0].is_direct_visible());
    assert_eq!(trace_2d.sources()[0].discovered_fraction(), 0.0);
    Ok(())
}
