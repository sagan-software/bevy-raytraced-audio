//! Public contract tests for the CPU acoustic core.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene2d, AcousticScene3d, BandAbsorption, BandEnergy, BandGain,
    Emitter2d, Emitter3d, GeometryError, Listener2d, Listener3d, Point2, Point3, Segment2d,
    Triangle3d,
};

/// An unobstructed two-dimensional path retains unity transmission.
#[test]
fn open_scene_preserves_direct_transmission() -> Result<(), GeometryError> {
    let scene = AcousticScene2d::default();
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(1.0, 0.0)?);

    let response = scene.trace(emitter, listener);

    assert_eq!(response.direct.distance_m(), 2.0);
    assert_eq!(response.direct.gain(), BandGain::UNITY);
    assert!(!response.direct.is_occluded());
    Ok(())
}

/// An explicit segment blocks the direct path through its interior.
#[test]
fn crossed_segment_blocks_direct_path() -> Result<(), GeometryError> {
    let wall = Segment2d::try_new(
        Point2::try_new(0.0, -1.0)?,
        Point2::try_new(0.0, 1.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(wall);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(1.0, 0.0)?);

    let response = scene.trace(emitter, listener);

    assert!(response.direct.is_occluded());
    assert_eq!(response.direct.gain(), BandGain::ZERO);
    Ok(())
}

/// A direct path multiplies each crossed segment's transmission independently per band.
#[test]
fn crossed_2d_surfaces_multiply_band_transmission() -> Result<(), GeometryError> {
    let first_material = AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.2, 0.3)?)
        .try_with_transmission(BandGain::try_new(0.5, 0.75, 0.8)?)?;
    let second_material = AcousticMaterial::new(BandAbsorption::try_new(0.4, 0.5, 0.6)?)
        .try_with_transmission(BandGain::try_new(0.4, 0.25, 0.3)?)?;
    let first_wall = Segment2d::try_new(
        Point2::try_new(-0.5, -1.0)?,
        Point2::try_new(-0.5, 1.0)?,
        first_material,
    )?;
    let second_wall = Segment2d::try_new(
        Point2::try_new(0.5, -1.0)?,
        Point2::try_new(0.5, 1.0)?,
        second_material,
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(first_wall);
    scene.add_segment(second_wall);
    for x in [10.0, 20.0, 30.0] {
        scene.add_segment(Segment2d::try_new(
            Point2::try_new(x, 5.0)?,
            Point2::try_new(x, 6.0)?,
            AcousticMaterial::default(),
        )?);
    }
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(1.0, 0.0)?);

    let response = scene.trace(emitter, listener);
    let gain = response.direct.gain();

    assert!(response.direct.is_occluded());
    assert!((gain.low() - 0.2).abs() < 1.0e-6);
    assert!((gain.mid() - 0.1875).abs() < 1.0e-6);
    assert!((gain.high() - 0.24).abs() < 1.0e-6);
    Ok(())
}

/// A direct path multiplies each crossed triangle's transmission independently per band.
#[test]
fn crossed_3d_surfaces_multiply_band_transmission() -> Result<(), GeometryError> {
    let first_material =
        AcousticMaterial::default().try_with_transmission(BandGain::try_new(0.5, 0.75, 1.0)?)?;
    let second_material =
        AcousticMaterial::default().try_with_transmission(BandGain::try_new(0.4, 0.25, 0.8)?)?;
    let first_wall = Triangle3d::try_new(
        [
            Point3::try_new(-0.5, -1.0, -1.0)?,
            Point3::try_new(-0.5, 1.0, -1.0)?,
            Point3::try_new(-0.5, 0.0, 1.0)?,
        ],
        first_material,
    )?;
    let second_wall = Triangle3d::try_new(
        [
            Point3::try_new(0.5, -1.0, -1.0)?,
            Point3::try_new(0.5, 1.0, -1.0)?,
            Point3::try_new(0.5, 0.0, 1.0)?,
        ],
        second_material,
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(first_wall);
    scene.add_triangle(second_wall);
    for x in [10.0, 20.0, 30.0] {
        scene.add_triangle(Triangle3d::try_new(
            [
                Point3::try_new(x, 5.0, -1.0)?,
                Point3::try_new(x, 6.0, -1.0)?,
                Point3::try_new(x, 5.5, 1.0)?,
            ],
            AcousticMaterial::default(),
        )?);
    }
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(1.0, 0.0, 0.0)?);

    let response = scene.trace(emitter, listener);
    let gain = response.direct.gain();

    assert!(response.direct.is_occluded());
    assert!((gain.low() - 0.2).abs() < 1.0e-6);
    assert!((gain.mid() - 0.1875).abs() < 1.0e-6);
    assert!((gain.high() - 0.8).abs() < 1.0e-6);
    Ok(())
}

/// A fully transmitting surface leaves the path unoccluded in both dimensions.
#[test]
fn fully_transmitting_surfaces_preserve_direct_visibility() -> Result<(), GeometryError> {
    let material = AcousticMaterial::default().try_with_transmission(BandGain::UNITY)?;
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(1.0, 0.0)?);
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(Segment2d::try_new(
        Point2::try_new(0.0, -1.0)?,
        Point2::try_new(0.0, 1.0)?,
        material,
    )?);
    let response2d = scene2d.trace(emitter2d, listener2d);

    assert!(!response2d.direct.is_occluded());
    assert_eq!(response2d.direct.gain(), BandGain::UNITY);

    let emitter3d = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(1.0, 0.0, 0.0)?);
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(Triangle3d::try_new(
        [
            Point3::try_new(0.0, -1.0, -1.0)?,
            Point3::try_new(0.0, 1.0, -1.0)?,
            Point3::try_new(0.0, 0.0, 1.0)?,
        ],
        material,
    )?);
    let response3d = scene3d.trace(emitter3d, listener3d);

    assert!(!response3d.direct.is_occluded());
    assert_eq!(response3d.direct.gain(), BandGain::UNITY);
    Ok(())
}

/// A material rejects per-band absorbed and transmitted energy above the incident budget.
#[test]
fn material_transmission_respects_absorption_energy_budget() -> Result<(), GeometryError> {
    for (absorption, transmission) in [
        (
            BandAbsorption::try_new(0.8, 0.0, 0.0)?,
            BandGain::try_new(0.5, 0.0, 0.0)?,
        ),
        (
            BandAbsorption::try_new(0.0, 0.8, 0.0)?,
            BandGain::try_new(0.0, 0.5, 0.0)?,
        ),
        (
            BandAbsorption::try_new(0.0, 0.0, 0.8)?,
            BandGain::try_new(0.0, 0.0, 0.5)?,
        ),
    ] {
        let material = AcousticMaterial::new(absorption);
        assert_eq!(material.absorption(), absorption);
        let result = material.try_with_transmission(transmission);
        assert_eq!(
            result,
            Err(GeometryError::MaterialEnergyExceedsIncidentEnergy)
        );
    }

    let valid_boundary = AcousticMaterial::new(BandAbsorption::try_new(0.75, 0.5, 0.0)?)
        .try_with_transmission(BandGain::try_new(0.5, 0.5, 1.0)?);
    assert!(valid_boundary.is_ok());

    // Each band rejects positive absorption alongside full transmission, including f32 subnormals.
    let smallest_positive_absorption = f32::from_bits(1);
    for (absorption, transmission) in [
        (
            BandAbsorption::try_new(smallest_positive_absorption, 0.0, 0.0)?,
            BandGain::try_new(1.0, 0.0, 0.0)?,
        ),
        (
            BandAbsorption::try_new(0.0, smallest_positive_absorption, 0.0)?,
            BandGain::try_new(0.0, 1.0, 0.0)?,
        ),
        (
            BandAbsorption::try_new(0.0, 0.0, smallest_positive_absorption)?,
            BandGain::try_new(0.0, 0.0, 1.0)?,
        ),
    ] {
        assert_eq!(
            AcousticMaterial::new(absorption).try_with_transmission(transmission),
            Err(GeometryError::MaterialEnergyExceedsIncidentEnergy)
        );
    }
    Ok(())
}

/// Adding and clearing surfaces after a trace keeps the public query current.
#[test]
fn scene_mutations_after_queries_update_both_dimensions() -> Result<(), GeometryError> {
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(1.0, 0.0)?);
    let wall2d = Segment2d::try_new(
        Point2::try_new(0.0, -1.0)?,
        Point2::try_new(0.0, 1.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene2d = AcousticScene2d::default();

    assert!(!scene2d.trace(emitter2d, listener2d).direct.is_occluded());
    scene2d.add_segment(wall2d);
    assert!(scene2d.trace(emitter2d, listener2d).direct.is_occluded());
    scene2d.clear();
    assert!(!scene2d.trace(emitter2d, listener2d).direct.is_occluded());

    let emitter3d = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(1.0, 0.0, 0.0)?);
    let wall3d = Triangle3d::try_new(
        [
            Point3::try_new(0.0, -1.0, -1.0)?,
            Point3::try_new(0.0, 1.0, -1.0)?,
            Point3::try_new(0.0, 0.0, 1.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene3d = AcousticScene3d::default();

    assert!(!scene3d.trace(emitter3d, listener3d).direct.is_occluded());
    scene3d.add_triangle(wall3d);
    assert!(scene3d.trace(emitter3d, listener3d).direct.is_occluded());
    scene3d.clear();
    assert!(!scene3d.trace(emitter3d, listener3d).direct.is_occluded());
    Ok(())
}

/// Spatially separated extra surfaces keep first-order path output exact in both dimensions.
#[test]
fn multi_surface_scenes_preserve_reflection_results() -> Result<(), GeometryError> {
    let material = AcousticMaterial::default();
    let reflector2d = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        material,
    )?;
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(reflector2d);
    for (x, y) in [(20.0, 100.0), (30.0, 200.0), (40.0, 300.0), (50.0, 400.0)] {
        scene2d.add_segment(Segment2d::try_new(
            Point2::try_new(x, y)?,
            Point2::try_new(x, y + 1.0)?,
            material,
        )?);
    }
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(-1.0, 2.0)?);
    let mut paths2d = Vec::new();
    let response2d = scene2d.trace_with_reflection_paths(emitter2d, listener2d, &mut paths2d);

    assert!(!response2d.direct.is_occluded());
    assert_eq!(paths2d.len(), 1);
    let Some(path2d) = paths2d.first() else {
        panic!("the finite 2D reflector must produce one first-order path");
    };
    assert_eq!(path2d.surface_index().index(), 0);
    assert!((path2d.distance_m() - 20.0_f64.sqrt()).abs() < 1.0e-6);
    assert_eq!(response2d.reflected_energy, path2d.relative_energy());

    let reflector3d = Triangle3d::try_new(
        [
            Point3::try_new(-2.0, -2.0, 1.0)?,
            Point3::try_new(2.0, -2.0, 1.0)?,
            Point3::try_new(0.0, 3.0, 1.0)?,
        ],
        material,
    )?;
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(reflector3d);
    for (x, y, z) in [
        (100.0, 100.0, 2.0),
        (200.0, 200.0, 3.0),
        (300.0, 300.0, 4.0),
        (400.0, 400.0, 5.0),
    ] {
        scene3d.add_triangle(Triangle3d::try_new(
            [
                Point3::try_new(x, y, z)?,
                Point3::try_new(x + 1.0, y, z)?,
                Point3::try_new(x, y + 1.0, z)?,
            ],
            material,
        )?);
    }
    let emitter3d = Emitter3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(0.0, 2.0, 0.0)?);
    let mut paths3d = Vec::new();
    let response3d = scene3d.trace_with_reflection_paths(emitter3d, listener3d, &mut paths3d);

    assert!(!response3d.direct.is_occluded());
    assert_eq!(paths3d.len(), 1);
    let Some(path3d) = paths3d.first() else {
        panic!("the finite 3D reflector must produce one first-order path");
    };
    assert_eq!(path3d.surface_index().index(), 0);
    assert!((path3d.distance_m() - 8.0_f64.sqrt()).abs() < 1.0e-6);
    assert_eq!(response3d.reflected_energy, path3d.relative_energy());
    Ok(())
}

/// A specular first-order 2D reflection contributes energy without blocking the direct path.
#[test]
fn same_side_segment_contributes_reflected_energy() -> Result<(), GeometryError> {
    let wall = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(wall);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(-1.0, 2.0)?);

    let response = scene.trace(emitter, listener);

    assert!(!response.direct.is_occluded());
    assert!(response.reflected_energy.low() > 0.0);
    assert!(response.reflected_energy.mid() > 0.0);
    assert!(response.reflected_energy.high() > 0.0);
    Ok(())
}

/// A reflection query returns geometric data in stable surface insertion order.
#[test]
fn reflection_paths_expose_surface_point_distance_and_energy() -> Result<(), GeometryError> {
    let surface = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::new(BandAbsorption::try_new(0.0, 0.0, 0.0)?),
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(surface);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(-1.0, 2.0)?);

    let paths: Vec<_> = scene.reflection_paths(emitter, listener).collect();

    assert_eq!(paths.len(), 1);
    let Some(path) = paths.first() else {
        panic!("the visible wall must produce one first-order reflection");
    };
    assert_eq!(path.surface_index().index(), 0);
    assert_eq!(path.reflection_point().x_m(), 1.0);
    assert_eq!(path.reflection_point().y_m(), 1.0);
    assert_eq!(path.image_source().x_m(), 3.0);
    assert_eq!(path.image_source().y_m(), 0.0);
    assert!((path.distance_m() - 20.0_f64.sqrt()).abs() < 1.0e-6);
    assert!((path.relative_energy().low() - 0.2).abs() < 1.0e-6);
    assert!((path.relative_energy().mid() - 0.2).abs() < 1.0e-6);
    assert!((path.relative_energy().high() - 0.2).abs() < 1.0e-6);
    assert_eq!(
        scene.trace(emitter, listener).reflected_energy,
        path.relative_energy()
    );
    Ok(())
}

/// A three-dimensional reflection query reports its triangle, virtual source, distance, and energy.
#[test]
fn three_dimensional_reflection_path_exposes_geometry_and_energy() -> Result<(), GeometryError> {
    let surface = Triangle3d::try_new(
        [
            Point3::try_new(-2.0, -2.0, 1.0)?,
            Point3::try_new(2.0, -2.0, 1.0)?,
            Point3::try_new(0.0, 3.0, 1.0)?,
        ],
        AcousticMaterial::new(BandAbsorption::try_new(0.0, 0.0, 0.0)?),
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(surface);
    let emitter = Emitter3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(0.0, 2.0, 0.0)?);

    let path = scene.reflection_paths(emitter, listener).next();

    let Some(path) = path else {
        panic!("the finite triangle must produce one first-order reflection");
    };
    assert_eq!(path.surface_index().index(), 0);
    assert_eq!(path.reflection_point().x_m(), 0.0);
    assert_eq!(path.reflection_point().y_m(), 1.0);
    assert_eq!(path.reflection_point().z_m(), 1.0);
    assert_eq!(path.image_source().x_m(), 0.0);
    assert_eq!(path.image_source().y_m(), 0.0);
    assert_eq!(path.image_source().z_m(), 2.0);
    assert!((path.distance_m() - 8.0_f64.sqrt()).abs() < 1.0e-6);
    assert!((path.relative_energy().low() - 0.5).abs() < 1.0e-6);
    assert!((path.relative_energy().mid() - 0.5).abs() < 1.0e-6);
    assert!((path.relative_energy().high() - 0.5).abs() < 1.0e-6);
    assert_eq!(
        scene.trace(emitter, listener).reflected_energy,
        path.relative_energy()
    );
    Ok(())
}

/// Both scene dimensions report reflection surface indices in insertion order.
#[test]
fn reflection_paths_follow_surface_insertion_order() -> Result<(), GeometryError> {
    let segment = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(segment);
    scene2d.add_segment(segment);
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(-1.0, 2.0)?);
    let indices2d: Vec<_> = scene2d
        .reflection_paths(emitter2d, listener2d)
        .map(|path| path.surface_index().index())
        .collect();

    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(-2.0, -2.0, 1.0)?,
            Point3::try_new(2.0, -2.0, 1.0)?,
            Point3::try_new(0.0, 3.0, 1.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(triangle);
    scene3d.add_triangle(triangle);
    let emitter3d = Emitter3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(0.0, 2.0, 0.0)?);
    let indices3d: Vec<_> = scene3d
        .reflection_paths(emitter3d, listener3d)
        .map(|path| path.surface_index().index())
        .collect();

    assert_eq!(indices2d, [0, 1]);
    assert_eq!(indices3d, [0, 1]);
    let aggregate2d = scene2d.trace(emitter2d, listener2d).reflected_energy;
    let aggregate3d = scene3d.trace(emitter3d, listener3d).reflected_energy;
    assert!((aggregate2d.low() - 0.4).abs() < 1.0e-6);
    assert!((aggregate2d.mid() - 0.4).abs() < 1.0e-6);
    assert!((aggregate2d.high() - 0.4).abs() < 1.0e-6);
    assert!((aggregate3d.low() - 1.0).abs() < 1.0e-6);
    assert!((aggregate3d.mid() - 1.0).abs() < 1.0e-6);
    assert!((aggregate3d.high() - 1.0).abs() < 1.0e-6);
    Ok(())
}

/// Coincident emitters and listeners have no finite reflected path in either scene dimension.
#[test]
fn coincident_source_and_listener_have_no_reflection_paths() -> Result<(), GeometryError> {
    let segment = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(segment);
    let point2 = Point2::try_new(-1.0, 0.0)?;
    assert!(
        scene2d
            .reflection_paths(Emitter2d::new(point2), Listener2d::new(point2))
            .next()
            .is_none()
    );

    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(-2.0, -2.0, 1.0)?,
            Point3::try_new(2.0, -2.0, 1.0)?,
            Point3::try_new(0.0, 3.0, 1.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(triangle);
    let point3 = Point3::try_new(0.0, 0.0, 0.0)?;
    assert!(
        scene3d
            .reflection_paths(Emitter3d::new(point3), Listener3d::new(point3))
            .next()
            .is_none()
    );
    Ok(())
}

/// A second segment blocks the reflector's source-to-surface leg.
#[test]
fn segment_blocks_a_reflected_path() -> Result<(), GeometryError> {
    let reflector = Segment2d::try_new(
        Point2::try_new(0.0, -2.0)?,
        Point2::try_new(0.0, 4.0)?,
        AcousticMaterial::default(),
    )?;
    let fully_absorbing = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0)?);
    let blocker = Segment2d::try_new(
        Point2::try_new(-0.75, 0.5)?,
        Point2::try_new(-0.25, 0.5)?,
        fully_absorbing,
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(reflector);
    scene.add_segment(blocker);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(-1.0, 2.0)?);

    let response = scene.trace(emitter, listener);

    assert!(!response.direct.is_occluded());
    assert_eq!(response.reflected_energy, BandEnergy::ZERO);
    let paths: Vec<_> = scene.reflection_paths(emitter, listener).collect();
    assert!(!paths.iter().any(|path| path.surface_index().index() == 0));
    Ok(())
}

/// Material absorption independently reduces reflected energy in each band.
#[test]
fn absorption_reduces_reflected_energy_per_band() -> Result<(), GeometryError> {
    let reflective = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    let absorption = BandAbsorption::try_new(1.0, 0.5, 0.0)?;
    let absorptive = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::new(absorption),
    )?;
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(-1.0, 2.0)?);
    let mut reflective_scene = AcousticScene2d::default();
    reflective_scene.add_segment(reflective);
    let mut absorptive_scene = AcousticScene2d::default();
    absorptive_scene.add_segment(absorptive);

    let reflective_energy = reflective_scene.trace(emitter, listener).reflected_energy;
    let absorptive_energy = absorptive_scene.trace(emitter, listener).reflected_energy;

    assert_eq!(absorptive_energy.low(), 0.0);
    assert!(absorptive_energy.mid() < reflective_energy.mid());
    assert_eq!(absorptive_energy.high(), reflective_energy.high());

    let paths: Vec<_> = absorptive_scene
        .reflection_paths(emitter, listener)
        .collect();
    let Some(path) = paths.first() else {
        panic!("the reflective wall must retain a visible path");
    };
    assert_eq!(path.relative_energy().low(), 0.0);
    assert!((path.relative_energy().mid() - 0.1).abs() < 1.0e-6);
    assert!((path.relative_energy().high() - 0.2).abs() < 1.0e-6);
    Ok(())
}

/// Reflected energy uses the energy left after absorption and squared transmission.
#[test]
fn transmission_reduces_reflected_energy_per_band() -> Result<(), GeometryError> {
    let absorption = BandAbsorption::try_new(0.1, 0.2, 0.3)?;
    let transmission = BandGain::try_new(0.5, 0.6, 0.7)?;
    let material = AcousticMaterial::new(absorption).try_with_transmission(transmission)?;
    let segment = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        material,
    )?;
    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(-2.0, -2.0, 1.0)?,
            Point3::try_new(2.0, -2.0, 1.0)?,
            Point3::try_new(0.0, 3.0, 1.0)?,
        ],
        material,
    )?;
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(segment);
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(triangle);
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(-1.0, 2.0)?);
    let emitter3d = Emitter3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(0.0, 2.0, 0.0)?);

    let energy2d = scene2d.trace(emitter2d, listener2d).reflected_energy;
    let energy3d = scene3d.trace(emitter3d, listener3d).reflected_energy;

    assert!((energy2d.low() - 0.13).abs() < 1.0e-6);
    assert!((energy2d.mid() - 0.088).abs() < 1.0e-6);
    assert!((energy2d.high() - 0.042).abs() < 1.0e-6);
    assert!((energy3d.low() - 0.325).abs() < 1.0e-6);
    assert!((energy3d.mid() - 0.22).abs() < 1.0e-6);
    assert!((energy3d.high() - 0.105).abs() < 1.0e-6);
    Ok(())
}

/// Non-finite coordinates are rejected at point construction.
#[test]
fn points_reject_non_finite_coordinates() {
    assert_eq!(
        Point2::try_new(f32::NAN, 0.0),
        Err(GeometryError::NonFiniteValue)
    );
    assert_eq!(
        Point3::try_new(0.0, f32::INFINITY, 0.0),
        Err(GeometryError::NonFiniteValue)
    );
}

/// Gains and absorption use bounded coefficients, while aggregate energy allows values above one.
#[test]
fn coefficient_and_energy_ranges_are_validated() -> Result<(), GeometryError> {
    assert_eq!(
        BandGain::try_new(1.0, -0.1, 1.0),
        Err(GeometryError::CoefficientOutOfRange)
    );
    assert_eq!(
        BandAbsorption::try_new(1.1, 0.0, 0.0),
        Err(GeometryError::CoefficientOutOfRange)
    );
    assert_eq!(
        BandEnergy::try_new(0.0, f64::NAN, 0.0),
        Err(GeometryError::NonFiniteValue)
    );
    assert_eq!(
        BandAbsorption::try_new(0.0, f32::INFINITY, 0.0),
        Err(GeometryError::NonFiniteValue)
    );
    assert_eq!(
        BandGain::try_new(f32::NAN, 0.0, 0.0),
        Err(GeometryError::NonFiniteValue)
    );
    assert_eq!(
        BandEnergy::try_new(0.0, -0.1, 0.0),
        Err(GeometryError::NegativeEnergy)
    );
    assert_eq!(BandEnergy::try_new(1.5, 0.0, 0.0)?.low(), 1.5);
    Ok(())
}

/// Gain multiplication preserves separate frequency-band values.
#[test]
fn gain_multiplication_is_per_band() -> Result<(), GeometryError> {
    let gain = BandGain::try_new(0.25, 0.5, 1.0)?;
    let second_gain = BandGain::try_new(0.5, 0.25, 0.75)?;
    let product = gain.multiply(second_gain);

    assert_eq!(gain.low(), 0.25);
    assert_eq!(gain.mid(), 0.5);
    assert_eq!(gain.high(), 1.0);
    assert_eq!(product.low(), 0.125);
    assert_eq!(product.mid(), 0.125);
    assert_eq!(product.high(), 0.75);
    assert_eq!(BandGain::default(), BandGain::UNITY);
    Ok(())
}

/// Each machine-readable geometry failure has a stable concise diagnostic.
#[test]
fn geometry_error_messages_cover_each_category() {
    assert_eq!(
        GeometryError::NonFiniteValue.to_string(),
        "value must be finite"
    );
    assert_eq!(
        GeometryError::CoefficientOutOfRange.to_string(),
        "coefficient must be in the inclusive range [0, 1]"
    );
    assert_eq!(
        GeometryError::MaterialEnergyExceedsIncidentEnergy.to_string(),
        "material absorption plus transmitted energy must not exceed 1"
    );
    assert_eq!(
        GeometryError::NegativeEnergy.to_string(),
        "reflected energy must be non-negative"
    );
    assert_eq!(
        GeometryError::DegeneratePrimitive.to_string(),
        "acoustic geometry primitive must have nonzero measure"
    );
}

/// Degenerate line segments and triangles cannot enter a scene.
#[test]
fn degenerate_primitives_are_rejected() -> Result<(), GeometryError> {
    let point2 = Point2::try_new(1.0, 2.0)?;
    assert_eq!(
        Segment2d::try_new(point2, point2, AcousticMaterial::default()),
        Err(GeometryError::DegeneratePrimitive)
    );

    let point3 = Point3::try_new(1.0, 2.0, 3.0)?;
    assert_eq!(
        Triangle3d::try_new([point3, point3, point3], AcousticMaterial::default()),
        Err(GeometryError::DegeneratePrimitive)
    );
    Ok(())
}

/// A triangle intersecting the direct path blocks 3D transmission.
#[test]
fn crossed_triangle_blocks_direct_path() -> Result<(), GeometryError> {
    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(0.0, -1.0, -1.0)?,
            Point3::try_new(0.0, 1.0, -1.0)?,
            Point3::try_new(0.0, 0.0, 1.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(triangle);
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(1.0, 0.0, 0.0)?);

    let response = scene.trace(emitter, listener);

    assert!(response.direct.is_occluded());
    assert_eq!(response.direct.gain(), BandGain::ZERO);
    Ok(())
}

/// An image-source path contributes aggregate 3D reflection energy.
#[test]
fn same_side_triangle_contributes_reflected_energy() -> Result<(), GeometryError> {
    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(1.0, -1.0, 0.0)?,
            Point3::try_new(1.0, 1.0, 0.0)?,
            Point3::try_new(1.0, 0.0, 2.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(triangle);
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(-1.0, 0.0, 2.0)?);

    let response = scene.trace(emitter, listener);

    assert!(!response.direct.is_occluded());
    assert!(response.reflected_energy.low() > 0.0);
    Ok(())
}

/// A second triangle blocks the reflector's source-to-surface leg.
#[test]
fn triangle_blocks_a_reflected_path() -> Result<(), GeometryError> {
    let reflector = Triangle3d::try_new(
        [
            Point3::try_new(0.0, -1.0, -1.0)?,
            Point3::try_new(0.0, 1.0, -1.0)?,
            Point3::try_new(0.0, 0.0, 3.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let fully_absorbing = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0)?);
    let blocker = Triangle3d::try_new(
        [
            Point3::try_new(-0.5, -0.5, 0.0)?,
            Point3::try_new(-0.5, 0.5, 0.0)?,
            Point3::try_new(-0.5, 0.0, 1.0)?,
        ],
        fully_absorbing,
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(reflector);
    scene.add_triangle(blocker);
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(-1.0, 0.0, 2.0)?);

    let response = scene.trace(emitter, listener);

    assert!(!response.direct.is_occluded());
    assert_eq!(response.reflected_energy, BandEnergy::ZERO);
    let paths: Vec<_> = scene.reflection_paths(emitter, listener).collect();
    assert!(!paths.iter().any(|path| path.surface_index().index() == 0));
    assert!(
        paths
            .iter()
            .all(|path| path.relative_energy() == BandEnergy::ZERO)
    );
    Ok(())
}

/// Scene clear methods remove all registered geometry, and zero-length paths stay finite.
#[test]
fn clearing_scenes_removes_geometry_and_zero_length_paths_are_valid() -> Result<(), GeometryError> {
    let segment = Segment2d::try_new(
        Point2::try_new(0.0, 0.0)?,
        Point2::try_new(1.0, 0.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene2d = AcousticScene2d::default();
    scene2d.add_segment(segment);
    assert_eq!(scene2d.segment_count(), 1);
    scene2d.clear();
    assert_eq!(scene2d.segment_count(), 0);
    let point2 = Point2::try_new(2.0, -1.0)?;
    let response2d = scene2d.trace(Emitter2d::new(point2), Listener2d::new(point2));
    assert_eq!(response2d.direct.distance_m(), 0.0);
    assert_eq!(response2d.reflected_energy, BandEnergy::ZERO);

    let triangle = Triangle3d::try_new(
        [
            Point3::try_new(0.0, 0.0, 0.0)?,
            Point3::try_new(1.0, 0.0, 0.0)?,
            Point3::try_new(0.0, 1.0, 0.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene3d = AcousticScene3d::default();
    scene3d.add_triangle(triangle);
    assert_eq!(scene3d.triangle_count(), 1);
    scene3d.clear();
    assert_eq!(scene3d.triangle_count(), 0);
    let point3 = Point3::try_new(2.0, -1.0, 4.0)?;
    let response3d = scene3d.trace(Emitter3d::new(point3), Listener3d::new(point3));
    assert_eq!(response3d.direct.distance_m(), 0.0);
    assert_eq!(response3d.reflected_energy, BandEnergy::ZERO);
    Ok(())
}

/// A validated GPU direct gain replaces only direct transmission in the CPU response.
#[test]
fn gpu_direct_gain_preserves_cpu_distance_and_reflections() -> Result<(), GeometryError> {
    let reflector = Triangle3d::try_new(
        [
            Point3::try_new(1.0, -1.0, 0.0)?,
            Point3::try_new(1.0, 1.0, 0.0)?,
            Point3::try_new(1.0, 0.0, 2.0)?,
        ],
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene3d::default();
    scene.add_triangle(reflector);
    let position = Point3::try_new(-1.0, 0.0, 0.0)?;
    let receiver = Point3::try_new(-1.0, 0.0, 2.0)?;
    let cpu_response = scene.trace(Emitter3d::new(position), Listener3d::new(receiver));
    let gpu_gain = BandGain::try_new(0.25, 0.5, 0.75)?;

    let response = cpu_response.with_direct_gain(gpu_gain);

    assert_eq!(
        response.direct.distance_m(),
        cpu_response.direct.distance_m()
    );
    assert_eq!(response.reflected_energy, cpu_response.reflected_energy);
    assert_eq!(response.direct.gain(), gpu_gain);
    assert!(response.direct.is_occluded());

    let unobstructed = response.with_direct_gain(BandGain::UNITY);
    assert_eq!(unobstructed.direct.gain(), BandGain::UNITY);
    assert!(!unobstructed.direct.is_occluded());
    Ok(())
}

/// Settings preserve legal boundaries and reject nonfinite scattering independently of geometry.
#[test]
fn listener_settings_and_material_scattering_contracts() -> Result<(), GeometryError> {
    use bevy_raytraced_audio::RayTraceSettings;
    let settings = RayTraceSettings::default()
        .try_with_ray_count(1)?
        .with_max_bounces(0)
        .try_with_escape_distance(0.001)?
        .try_with_scattering(1.)?
        .with_permeation_ray_count(0)
        .with_seed(7);
    assert_eq!(settings.ray_count(), 1);
    assert_eq!(settings.max_bounces(), 0);
    assert_eq!(settings.permeation_ray_count(), 0);
    assert_eq!(settings.seed(), 7);
    assert_eq!(settings.escape_distance_m(), 0.001);
    assert_eq!(settings.scattering(), 1.);
    assert!(settings.try_with_scattering(f32::NAN).is_err());
    assert!(settings.try_with_scattering(-0.1).is_err());
    for scattering in [0., 0.5, 1.] {
        assert_eq!(
            AcousticMaterial::default()
                .try_with_scattering(scattering)?
                .scattering(),
            Some(scattering)
        );
    }
    assert!(
        AcousticMaterial::default()
            .try_with_scattering(f32::INFINITY)
            .is_err()
    );
    assert!(
        AcousticMaterial::default()
            .try_with_scattering(-0.1)
            .is_err()
    );
    Ok(())
}

/// Reused trace storage remains correct when voices and geometry disappear together.
#[test]
fn recorded_listener_traces_survive_bulk_removal() -> Result<(), GeometryError> {
    use bevy_raytraced_audio::{ListenerTrace2d, ListenerTrace3d, RayTraceSettings};
    let settings = RayTraceSettings::default()
        .with_recorded_rays(true)
        .try_with_ray_count(8)?;
    let mut scene2 = AcousticScene2d::default();
    let mut scene3 = AcousticScene3d::default();
    scene2.add_segment(Segment2d::try_new(
        Point2::try_new(1., -2.)?,
        Point2::try_new(1., 2.)?,
        AcousticMaterial::default(),
    )?);
    scene3.add_triangle(Triangle3d::try_new(
        [
            Point3::try_new(1., -2., -2.)?,
            Point3::try_new(1., 2., -2.)?,
            Point3::try_new(1., 0., 2.)?,
        ],
        AcousticMaterial::default(),
    )?);
    let l2 = Listener2d::new(Point2::try_new(0., 0.)?);
    let l3 = Listener3d::new(Point3::try_new(0., 0., 0.)?);
    let sources2 = vec![Emitter2d::new(Point2::try_new(2., 0.)?); 1024];
    let sources3 = vec![Emitter3d::new(Point3::try_new(2., 0., 0.)?); 1024];
    let mut out2 = ListenerTrace2d::default();
    let mut out3 = ListenerTrace3d::default();
    scene2.trace_listener(l2, &sources2, settings, &mut out2);
    scene3.trace_listener(l3, &sources3, settings, &mut out3);
    for segment in out2.segments() {
        assert!(segment.start().x_m().is_finite() && segment.end().y_m().is_finite());
        assert!(segment.start_distance_m() >= 0. && segment.length_m().is_finite());
        assert!((0. ..=1.).contains(&segment.energy()));
        assert!(
            segment
                .source_index()
                .is_none_or(|index| index < sources2.len())
        );
    }
    for segment in out3.segments() {
        assert!(segment.start().x_m().is_finite() && segment.end().z_m().is_finite());
        assert!(segment.start_distance_m() >= 0. && segment.length_m().is_finite());
        assert!((0. ..=1.).contains(&segment.energy()));
        assert!(
            segment
                .source_index()
                .is_none_or(|index| index < sources3.len())
        );
    }
    for response in out2.sources().iter().chain(out3.sources()) {
        assert!((0. ..=1.).contains(&response.muffle_strength()));
    }
    for reverb in [out2.reverb(), out3.reverb()] {
        assert!(reverb.mean_return_distance_m().is_finite());
        assert!(reverb.mean_free_path_m().is_finite());
    }
    scene2.clear();
    scene3.clear();
    scene2.trace_listener(l2, &[], settings, &mut out2);
    scene3.trace_listener(l3, &[], settings, &mut out3);
    assert!(out2.sources().is_empty() && out3.sources().is_empty());
    assert!(out2.segments().iter().all(|s| s.source_index().is_none()));
    assert!(out3.segments().iter().all(|s| s.source_index().is_none()));
    assert_eq!(out2.reverb().outdoor_fraction(), 1.);
    assert_eq!(out3.reverb().outdoor_fraction(), 1.);
    Ok(())
}

/// Shared DSP readback tracks updates and bypass without changing the stored target filter.
#[test]
fn dsp_control_readback_and_bounds() -> Result<(), GeometryError> {
    use bevy_raytraced_audio::{AcousticDspParams, MuffleFilter, ReverbEstimate};
    let params = AcousticDspParams::default();
    assert!(params.is_enabled());
    let filter = MuffleFilter::try_new(0.6, 0.2)?;
    params.set_filter(filter);
    assert_eq!(params.filter(), filter);
    params.set_reverb(ReverbEstimate::from_parameters(0.8, 2., 0.1), 0.5);
    assert!((params.wet_gain() - 0.4).abs() < 1.0e-6);
    params.set_enabled(false);
    assert!(!params.is_enabled());
    assert_eq!(params.filter(), filter);
    params.set_reverb(ReverbEstimate::DRY, f32::NAN);
    assert_eq!(params.wet_gain(), 0.);
    Ok(())
}

/// Public queries retain exact results for empty, parallel and oblique reflection groups.
#[test]
fn reflection_broad_phase_public_query_boundaries() -> Result<(), GeometryError> {
    let emitter2 = Emitter2d::new(Point2::try_new(-1., -1.)?);
    let listener2 = Listener2d::new(Point2::try_new(1., -1.)?);
    assert_eq!(
        AcousticScene2d::default()
            .reflection_paths(emitter2, listener2)
            .count(),
        0
    );
    for horizontal in [false, true] {
        let mut scene = AcousticScene2d::default();
        for value in 2_u16..10 {
            let value = f32::from(value);
            let (start, end) = if horizontal {
                ((-2., value), (2., value))
            } else {
                ((value, -2.), (value, 2.))
            };
            scene.add_segment(Segment2d::try_new(
                Point2::try_new(start.0, start.1)?,
                Point2::try_new(end.0, end.1)?,
                AcousticMaterial::default(),
            )?);
        }
        assert!(scene.reflection_paths(emitter2, listener2).count() > 0);
        let distant = Emitter2d::new(Point2::try_new(100., 100.)?);
        let receiver = Listener2d::new(Point2::try_new(101., 101.)?);
        assert_eq!(scene.reflection_paths(distant, receiver).count(), 0);
    }
    let emitter3 = Emitter3d::new(Point3::try_new(-1., -1., -1.)?);
    let listener3 = Listener3d::new(Point3::try_new(0., 0., 0.)?);
    assert_eq!(
        AcousticScene3d::default()
            .reflection_paths(emitter3, listener3)
            .count(),
        0
    );
    for axis in 0..3 {
        let mut scene = AcousticScene3d::default();
        for value in 2_u16..10 {
            let value = f32::from(value);
            let points = match axis {
                0 => [(value, -3., -3.), (value, 3., -3.), (value, 0., 3.)],
                1 => [(-3., value, -3.), (3., value, -3.), (0., value, 3.)],
                _ => [(-3., -3., value), (3., -3., value), (0., 3., value)],
            };
            scene.add_triangle(Triangle3d::try_new(
                points.map(|(x, y, z)| Point3::try_new(x, y, z).expect("finite triangle")),
                AcousticMaterial::default(),
            )?);
        }
        assert!(scene.reflection_paths(emitter3, listener3).count() > 0);
        let distant = Emitter3d::new(Point3::try_new(100., 100., 100.)?);
        let receiver = Listener3d::new(Point3::try_new(101., 101., 101.)?);
        assert_eq!(scene.reflection_paths(distant, receiver).count(), 0);
    }
    let estimate = bevy_raytraced_audio::ReverbEstimate::from_parameters(
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    );
    let estimate = estimate.with_band_decay(f32::NAN, f32::INFINITY);
    assert_eq!(estimate.decay_low_s(), 0.05);
    assert_eq!(estimate.decay_high_s(), 0.05);
    assert_eq!(estimate.wet_gain(), 0.);
    assert_eq!(estimate.decay_time_s(), 0.05);
    assert_eq!(estimate.reflections_delay_s(), 0.);
    Ok(())
}
