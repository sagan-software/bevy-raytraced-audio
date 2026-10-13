//! Criterion workloads for direct paths, reflections, and scene-size stress.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene2d, AcousticScene3d, Emitter2d, Emitter3d, GeometryError,
    Listener2d, Listener3d, ListenerTrace2d, ListenerTrace3d, Point2, Point3, RayTraceSettings,
    Segment2d, Triangle3d,
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
    benchmark_3d_reflection_limit(&mut criterion)?;
    benchmark_listener_frame(&mut criterion)?;
    benchmark_emitter_refresh_frame(&mut criterion)?;
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

/// Measures first-order reflections over a large floor with and without a reflection limit.
fn benchmark_3d_reflection_limit(criterion: &mut Criterion) -> Result<(), GeometryError> {
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 1.5, 0.0)?);
    let listener = Listener3d::new(Point3::try_new(1.0, 1.5, 0.0)?);
    let mut group = criterion.benchmark_group("propagation/3d/reflection_limit");
    for side in [32_u16, 128] {
        let unlimited = floor_scene3d(side)?;
        let limited = unlimited.clone().try_with_max_reflection_distance(30.0)?;
        let triangles = u64::try_from(unlimited.triangle_count()).unwrap_or(u64::MAX);
        group.throughput(Throughput::Elements(triangles));
        for (name, scene) in [("unlimited", &unlimited), ("limit_30m", &limited)] {
            group.bench_with_input(
                BenchmarkId::new(name, triangles),
                scene,
                |bencher, scene| {
                    bencher
                        .iter(|| black_box(scene.trace(black_box(emitter), black_box(listener))));
                },
            );
        }
    }
    group.finish();
    Ok(())
}

/// Creates a floor of `side` by `side` two-triangle tiles, each 4 m wide, centred on the origin.
fn floor_scene3d(side: u16) -> Result<AcousticScene3d, GeometryError> {
    let mut scene = AcousticScene3d::default();
    let half = f32::from(side) * 2.0;
    for row in 0..side {
        for column in 0..side {
            let tile_x = f32::from(column).mul_add(4.0, -half);
            let tile_z = f32::from(row).mul_add(4.0, -half);
            let corner = |dx: f32, dz: f32| Point3::try_new(tile_x + dx, 0.0, tile_z + dz);
            let [near_left, near_right, far_right, far_left] = [
                corner(0.0, 0.0)?,
                corner(4.0, 0.0)?,
                corner(4.0, 4.0)?,
                corner(0.0, 4.0)?,
            ];
            scene.add_triangle(Triangle3d::try_new(
                [near_left, near_right, far_right],
                AcousticMaterial::default(),
            )?);
            scene.add_triangle(Triangle3d::try_new(
                [near_left, far_right, far_left],
                AcousticMaterial::default(),
            )?);
        }
    }
    Ok(scene)
}

/// Measures full listener traces per frame with a moving listener and sources.
///
/// These are the acoustic costs a frame budget has to carry whenever a trace is due:
/// the default 256 rays and 8 bounces, with occlusion, echo, and permeation checks
/// against every source. Movement invalidates the completed-result cache each
/// iteration, so every sample measures a full recomputation.
fn benchmark_listener_frame(criterion: &mut Criterion) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("propagation/listener_frame");
    for source_count in [64_usize, 1024] {
        group.throughput(Throughput::Elements(
            u64::try_from(source_count).unwrap_or(u64::MAX),
        ));

        let scene2d = dense_scene2d(1024)?;
        let mut sources2d = dense_sources2d(source_count)?;
        let mut output2d = ListenerTrace2d::default();
        let mut frame2d = 0_usize;
        group.bench_with_input(
            BenchmarkId::new("2d/sources", source_count),
            &source_count,
            |bencher, _| {
                bencher.iter(|| {
                    frame2d += 1;
                    let shift = movement_shift(frame2d);
                    let listener = Listener2d::new(Point2::try_new(shift, 0.0).expect("finite"));
                    for (index, source) in sources2d.iter_mut().enumerate() {
                        *source = dense_source2d(index, shift).expect("finite fixture");
                    }
                    scene2d.trace_listener(
                        black_box(listener),
                        black_box(&sources2d),
                        RayTraceSettings::default(),
                        &mut output2d,
                    );
                    black_box(&output2d);
                    trace_checksum2d(&output2d)
                });
            },
        );

        let scene3d = dense_scene3d(1024)?;
        let mut sources3d = dense_sources3d(source_count)?;
        let mut output3d = ListenerTrace3d::default();
        let mut frame3d = 0_usize;
        group.bench_with_input(
            BenchmarkId::new("3d/sources", source_count),
            &source_count,
            |bencher, _| {
                bencher.iter(|| {
                    frame3d += 1;
                    let shift = movement_shift(frame3d);
                    let listener =
                        Listener3d::new(Point3::try_new(shift, 0.0, 0.0).expect("finite"));
                    for (index, source) in sources3d.iter_mut().enumerate() {
                        *source = dense_source3d(index, shift).expect("finite fixture");
                    }
                    scene3d.trace_listener(
                        black_box(listener),
                        black_box(&sources3d),
                        RayTraceSettings::default(),
                        &mut output3d,
                    );
                    black_box(&output3d);
                    trace_checksum3d(&output3d)
                });
            },
        );
    }
    group.finish();
    Ok(())
}

/// Measures one complete per-emitter image-source reflection refresh.
///
/// The adapters recompute every emitter's direct and first-order reflected
/// response whenever a trace is due; this is that sweep for a dense scene.
fn benchmark_emitter_refresh_frame(criterion: &mut Criterion) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("propagation/emitter_refresh");
    for emitter_count in [256_usize, 1024] {
        group.throughput(Throughput::Elements(
            u64::try_from(emitter_count).unwrap_or(u64::MAX),
        ));

        let scene2d = dense_scene2d(1024)?;
        let sources2d = dense_sources2d(emitter_count)?;
        let listener2d = Listener2d::new(Point2::try_new(0.0, 0.0)?);
        group.bench_with_input(
            BenchmarkId::new("2d/emitters", emitter_count),
            &emitter_count,
            |bencher, _| {
                bencher.iter(|| {
                    let mut checksum = 0.0;
                    for source in &sources2d {
                        let response = scene2d.trace(black_box(*source), black_box(listener2d));
                        checksum += response.direct.distance_m();
                    }
                    black_box(checksum)
                });
            },
        );

        let scene3d = dense_scene3d(1024)?;
        let limited3d = scene3d.clone().try_with_max_reflection_distance(30.0)?;
        let sources3d = dense_sources3d(emitter_count)?;
        let listener3d = Listener3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
        for (name, scene) in [("unlimited", &scene3d), ("limit_30m", &limited3d)] {
            group.bench_with_input(
                BenchmarkId::new(format!("3d/emitters/{name}"), emitter_count),
                &emitter_count,
                |bencher, _| {
                    bencher.iter(|| {
                        let mut checksum = 0.0;
                        for source in &sources3d {
                            let response = scene.trace(black_box(*source), black_box(listener3d));
                            checksum += response.direct.distance_m();
                        }
                        black_box(checksum)
                    });
                },
            );
        }
    }
    group.finish();
    Ok(())
}

/// Returns the deterministic per-frame movement offset for a frame counter.
fn movement_shift(frame: usize) -> f32 {
    f32::from(u16::try_from(frame % 32).unwrap_or(u16::MAX)) * 0.002
}

/// Consumes a listener trace into a finite checksum, matching the shared workloads.
fn trace_checksum2d(output: &ListenerTrace2d) -> f64 {
    f64::from(output.reverb().outdoor_fraction())
        + output
            .sources()
            .iter()
            .map(|source| f64::from(source.filter().gain_hf()))
            .sum::<f64>()
}

/// Consumes a listener trace into a finite checksum, matching the shared workloads.
fn trace_checksum3d(output: &ListenerTrace3d) -> f64 {
    f64::from(output.reverb().outdoor_fraction())
        + output
            .sources()
            .iter()
            .map(|source| f64::from(source.filter().gain_hf()))
            .sum::<f64>()
}

/// Converts a bounded fixture index into exact `f32` coordinates.
fn fixture_number(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("bounded fixture"))
}

/// Creates an enclosed 2D room with `count` short wall segments on a 0.25 m grid.
fn dense_scene2d(count: usize) -> Result<AcousticScene2d, GeometryError> {
    let mut scene = AcousticScene2d::default();
    for (a, b) in [
        ((-10.0, -10.0), (10.0, -10.0)),
        ((10.0, -10.0), (10.0, 10.0)),
        ((10.0, 10.0), (-10.0, 10.0)),
        ((-10.0, 10.0), (-10.0, -10.0)),
    ] {
        scene.add_segment(Segment2d::try_new(
            Point2::try_new(a.0, a.1)?,
            Point2::try_new(b.0, b.1)?,
            AcousticMaterial::default(),
        )?);
    }
    for index in 0..count {
        let x = fixture_number(index % 32).mul_add(0.25, -4.0);
        let y = fixture_number(index / 32).mul_add(0.25, -4.0);
        scene.add_segment(Segment2d::try_new(
            Point2::try_new(x, y)?,
            Point2::try_new(x, y + 0.24)?,
            AcousticMaterial::default(),
        )?);
    }
    Ok(scene)
}

/// Creates an enclosed 3D room with `count` baffle triangles on a 0.25 m grid.
fn dense_scene3d(count: usize) -> Result<AcousticScene3d, GeometryError> {
    let mut scene = AcousticScene3d::default();
    for [a, b, c, d] in [
        [
            [-10., -10., -10.],
            [10., -10., -10.],
            [10., 10., -10.],
            [-10., 10., -10.],
        ],
        [
            [-10., -10., 10.],
            [10., -10., 10.],
            [10., 10., 10.],
            [-10., 10., 10.],
        ],
        [
            [-10., -10., -10.],
            [-10., 10., -10.],
            [-10., 10., 10.],
            [-10., -10., 10.],
        ],
        [
            [10., -10., -10.],
            [10., 10., -10.],
            [10., 10., 10.],
            [10., -10., 10.],
        ],
        [
            [-10., -10., -10.],
            [10., -10., -10.],
            [10., -10., 10.],
            [-10., -10., 10.],
        ],
        [
            [-10., 10., -10.],
            [10., 10., -10.],
            [10., 10., 10.],
            [-10., 10., 10.],
        ],
    ] {
        for vertices in [[a, b, c], [a, c, d]] {
            let [first, second, third] = vertices.map(|[x, y, z]| Point3::try_new(x, y, z));
            scene.add_triangle(Triangle3d::try_new(
                [first?, second?, third?],
                AcousticMaterial::default(),
            )?);
        }
    }
    for index in 0..count {
        let x = fixture_number(index % 32).mul_add(0.25, -4.0);
        let z = fixture_number(index / 32).mul_add(0.25, -4.0);
        scene.add_triangle(Triangle3d::try_new(
            [
                Point3::try_new(x, -0.12, z)?,
                Point3::try_new(x, 0.12, z)?,
                Point3::try_new(x, 0.12, z + 0.24)?,
            ],
            AcousticMaterial::default(),
        )?);
    }
    Ok(scene)
}

/// Returns one dense-fixture 2D source shifted by `shift` meters along x.
fn dense_source2d(index: usize, shift: f32) -> Result<Emitter2d, GeometryError> {
    let x = fixture_number(index).mul_add(0.013, -6.0) + shift;
    let y = (fixture_number(index) * 0.071).sin() * 3.0;
    Ok(Emitter2d::new(Point2::try_new(x, y)?))
}

/// Returns one dense-fixture 3D source shifted by `shift` meters along x.
fn dense_source3d(index: usize, shift: f32) -> Result<Emitter3d, GeometryError> {
    let x = fixture_number(index).mul_add(0.013, -6.0) + shift;
    let y = (fixture_number(index) * 0.071).sin() * 3.0;
    let z = (fixture_number(index) * 0.047).cos() * 2.0;
    Ok(Emitter3d::new(Point3::try_new(x, y, z)?))
}

/// Creates the dense-fixture 2D source list.
fn dense_sources2d(count: usize) -> Result<Vec<Emitter2d>, GeometryError> {
    (0..count).map(|index| dense_source2d(index, 0.0)).collect()
}

/// Creates the dense-fixture 3D source list.
fn dense_sources3d(count: usize) -> Result<Vec<Emitter3d>, GeometryError> {
    (0..count).map(|index| dense_source3d(index, 0.0)).collect()
}
