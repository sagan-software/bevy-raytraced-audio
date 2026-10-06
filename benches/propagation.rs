//! Criterion workloads for direct paths, reflections, and scene-size stress.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene2d, AcousticScene3d, Emitter2d, Emitter3d, GeometryError,
    Listener2d, Listener3d, Point2, Point3, Segment2d, Triangle3d,
};
use criterion::{BenchmarkId, Criterion, Throughput};
use std::{hint::black_box, time::Duration};

/// Builds fixtures, then measures steady-state propagation queries.
fn main() -> Result<(), GeometryError> {
    let emitter2d = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener2d = Listener2d::new(Point2::try_new(-1.0, 2.0)?);
    let emitter3d = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0)?);
    let listener3d = Listener3d::new(Point3::try_new(-1.0, 0.0, 2.0)?);
    let empty2d = AcousticScene2d::default();
    let empty3d = AcousticScene3d::default();
    let blocked2d = blocked_scene2d()?;
    let blocked3d = blocked_scene3d()?;
    let reflected2d = reflected_scene2d()?;
    let reflected3d = reflected_scene3d()?;

    let mut criterion = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(500))
        .configure_from_args();

    benchmark_small_scenes(
        &mut criterion,
        &empty2d,
        &blocked2d,
        &reflected2d,
        emitter2d,
        listener2d,
        &empty3d,
        &blocked3d,
        &reflected3d,
        emitter3d,
        listener3d,
    );
    benchmark_2d_stress(&mut criterion, emitter2d, listener2d)?;
    benchmark_3d_stress(&mut criterion, emitter3d, listener3d)?;
    criterion.final_summary();
    Ok(())
}

/// Measures baseline, occluded, and reflected traces in both dimensions.
fn benchmark_small_scenes(
    criterion: &mut Criterion,
    empty2d: &AcousticScene2d,
    blocked2d: &AcousticScene2d,
    reflected2d: &AcousticScene2d,
    emitter2d: Emitter2d,
    listener2d: Listener2d,
    empty3d: &AcousticScene3d,
    blocked3d: &AcousticScene3d,
    reflected3d: &AcousticScene3d,
    emitter3d: Emitter3d,
    listener3d: Listener3d,
) {
    let mut group = criterion.benchmark_group("propagation/single_source_listener");
    group.bench_function("2d/direct_open", |bencher| {
        bencher.iter(|| black_box(empty2d.trace(black_box(emitter2d), black_box(listener2d))));
    });
    group.bench_function("2d/direct_occluded", |bencher| {
        bencher.iter(|| black_box(blocked2d.trace(black_box(emitter2d), black_box(listener2d))));
    });
    group.bench_function("2d/first_order_reflection", |bencher| {
        bencher.iter(|| black_box(reflected2d.trace(black_box(emitter2d), black_box(listener2d))));
    });
    group.bench_function("3d/direct_open", |bencher| {
        bencher.iter(|| black_box(empty3d.trace(black_box(emitter3d), black_box(listener3d))));
    });
    group.bench_function("3d/direct_occluded", |bencher| {
        bencher.iter(|| black_box(blocked3d.trace(black_box(emitter3d), black_box(listener3d))));
    });
    group.bench_function("3d/first_order_reflection", |bencher| {
        bencher.iter(|| black_box(reflected3d.trace(black_box(emitter3d), black_box(listener3d))));
    });
    group.finish();

    let mut reflection_group = criterion.benchmark_group("propagation/reflection_path_iteration");
    reflection_group.bench_function("2d/single_reflector", |bencher| {
        bencher.iter(|| {
            black_box(
                reflected2d
                    .reflection_paths(black_box(emitter2d), black_box(listener2d))
                    .count(),
            )
        });
    });
    reflection_group.bench_function("3d/single_reflector", |bencher| {
        bencher.iter(|| {
            black_box(
                reflected3d
                    .reflection_paths(black_box(emitter3d), black_box(listener3d))
                    .count(),
            )
        });
    });
    let mut paths2d = Vec::with_capacity(1);
    reflection_group.bench_function("2d/single_reflector_with_path_output", |bencher| {
        bencher.iter(|| {
            let response = reflected2d.trace_with_reflection_paths(
                black_box(emitter2d),
                black_box(listener2d),
                &mut paths2d,
            );
            black_box((&response, &paths2d));
        });
    });
    let mut paths3d = Vec::with_capacity(1);
    reflection_group.bench_function("3d/single_reflector_with_path_output", |bencher| {
        bencher.iter(|| {
            let response = reflected3d.trace_with_reflection_paths(
                black_box(emitter3d),
                black_box(listener3d),
                &mut paths3d,
            );
            black_box((&response, &paths3d));
        });
    });
    reflection_group.finish();
}

/// Measures full-scene misses as 2D wall count grows.
fn benchmark_2d_stress(
    criterion: &mut Criterion,
    emitter: Emitter2d,
    listener: Listener2d,
) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("propagation/2d/stress_miss");
    for surface_count in [32_usize, 256, 1024] {
        let scene = stress_scene2d(surface_count)?;
        group.throughput(Throughput::Elements(
            u64::try_from(surface_count).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(surface_count),
            &scene,
            |bencher, scene| {
                bencher.iter(|| black_box(scene.trace(black_box(emitter), black_box(listener))));
            },
        );
    }
    group.finish();
    Ok(())
}

/// Measures full-scene misses as 3D triangle count grows.
fn benchmark_3d_stress(
    criterion: &mut Criterion,
    emitter: Emitter3d,
    listener: Listener3d,
) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("propagation/3d/stress_miss");
    for surface_count in [32_usize, 256, 1024] {
        let scene = stress_scene3d(surface_count)?;
        group.throughput(Throughput::Elements(
            u64::try_from(surface_count).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(surface_count),
            &scene,
            |bencher, scene| {
                bencher.iter(|| black_box(scene.trace(black_box(emitter), black_box(listener))));
            },
        );
    }
    group.finish();
    Ok(())
}

/// Creates a direct-path blocker for the 2D reference benchmark.
fn blocked_scene2d() -> Result<AcousticScene2d, GeometryError> {
    let segment = Segment2d::try_new(
        Point2::try_new(0.0, -1.0)?,
        Point2::try_new(0.0, 1.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(segment);
    Ok(scene)
}

/// Creates a direct-path blocker for the 3D reference benchmark.
fn blocked_scene3d() -> Result<AcousticScene3d, GeometryError> {
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
    Ok(scene)
}

/// Creates a first-order specular reflector for the 2D reference benchmark.
fn reflected_scene2d() -> Result<AcousticScene2d, GeometryError> {
    let segment = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    let mut scene = AcousticScene2d::default();
    scene.add_segment(segment);
    Ok(scene)
}

/// Creates a first-order specular reflector for the 3D reference benchmark.
fn reflected_scene3d() -> Result<AcousticScene3d, GeometryError> {
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
    Ok(scene)
}

/// Creates 2D surfaces that all miss the source-listener path and reflection points.
fn stress_scene2d(surface_count: usize) -> Result<AcousticScene2d, GeometryError> {
    let mut scene = AcousticScene2d::default();
    let mut x = 10.0;
    for _ in 0..surface_count {
        let segment = Segment2d::try_new(
            Point2::try_new(x, 1000.0)?,
            Point2::try_new(x, 1001.0)?,
            AcousticMaterial::default(),
        )?;
        scene.add_segment(segment);
        x += 0.01;
    }
    Ok(scene)
}

/// Creates 3D surfaces that all miss the source-listener path and reflection points.
fn stress_scene3d(surface_count: usize) -> Result<AcousticScene3d, GeometryError> {
    let mut scene = AcousticScene3d::default();
    let mut x = 10.0;
    for _ in 0..surface_count {
        let triangle = Triangle3d::try_new(
            [
                Point3::try_new(x, 1000.0, 0.0)?,
                Point3::try_new(x, 1001.0, 0.0)?,
                Point3::try_new(x, 1000.0, 1.0)?,
            ],
            AcousticMaterial::default(),
        )?;
        scene.add_triangle(triangle);
        x += 0.01;
    }
    Ok(scene)
}
