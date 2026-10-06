//! Measures complete warmed Bevy acoustic updates for source and surface counts.

use bevy::prelude::{App, Transform, TransformPlugin, Vec2, Vec3};
use bevy_raytraced_audio::{AcousticMaterial, GeometryError};
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
    RaytracedAudioSurface2d,
};
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioSurface3d,
};
use criterion::{BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

/// Runs Criterion's complete Bevy-schedule benchmark groups.
fn main() -> Result<(), GeometryError> {
    let mut criterion = Criterion::default().configure_from_args();
    benchmark_2d(&mut criterion)?;
    benchmark_3d(&mut criterion)?;
    criterion.final_summary();
    Ok(())
}

/// Measures both single-source and many-source 2D adapter updates.
fn benchmark_2d(criterion: &mut Criterion) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("adapter_schedule/2d");
    for (emitter_count, surface_count) in [
        (1_usize, 0_usize),
        (1, 1),
        (64, 64),
        (128, 256),
        (256, 1024),
    ] {
        let mut app = app_2d(emitter_count, surface_count)?;
        app.update();
        group.throughput(Throughput::Elements(
            u64::try_from(emitter_count).unwrap_or(u64::MAX),
        ));
        let id = BenchmarkId::new(
            "sources_surfaces",
            format!("{emitter_count}_{surface_count}"),
        );
        group.bench_function(id, |bencher| {
            bencher.iter(|| black_box(&mut app).update());
        });
    }
    group.finish();
    Ok(())
}

/// Measures both single-source and many-source 3D adapter updates.
fn benchmark_3d(criterion: &mut Criterion) -> Result<(), GeometryError> {
    let mut group = criterion.benchmark_group("adapter_schedule/3d");
    for (emitter_count, surface_count) in [
        (1_usize, 0_usize),
        (1, 1),
        (64, 64),
        (128, 256),
        (256, 1024),
    ] {
        let mut app = app_3d(emitter_count, surface_count)?;
        app.update();
        group.throughput(Throughput::Elements(
            u64::try_from(emitter_count).unwrap_or(u64::MAX),
        ));
        let id = BenchmarkId::new(
            "sources_surfaces",
            format!("{emitter_count}_{surface_count}"),
        );
        group.bench_function(id, |bencher| {
            bencher.iter(|| black_box(&mut app).update());
        });
    }
    group.finish();
    Ok(())
}

/// Builds a warmed headless 2D Bevy app with deterministic geometry.
fn app_2d(emitter_count: usize, surface_count: usize) -> Result<App, GeometryError> {
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(0.0, 0.0, 0.0)));

    for emitter_index in 0..emitter_count {
        let emitter_offset = fixture_index(emitter_index);
        let x = emitter_offset.mul_add(0.013, -6.0);
        let y = (emitter_offset * 0.071).sin() * 3.0;
        app.world_mut()
            .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(x, y, 0.0)));
    }

    for surface_index in 0..surface_count {
        let column = surface_index % 32;
        let row = surface_index / 32;
        let x = fixture_index(column).mul_add(0.25, -4.0);
        let y = fixture_index(row).mul_add(0.25, -4.0);
        app.world_mut().spawn((
            RaytracedAudioSurface2d::new(
                Vec2::new(0.0, -0.12),
                Vec2::new(0.0, 0.12),
                AcousticMaterial::default(),
            )?,
            Transform::from_xyz(x, y, 0.0),
        ));
    }
    Ok(app)
}

/// Builds a warmed headless 3D Bevy app with deterministic triangles.
fn app_3d(emitter_count: usize, surface_count: usize) -> Result<App, GeometryError> {
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio3dPlugin::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener3d, Transform::from_xyz(0.0, 0.0, 0.0)));

    for emitter_index in 0..emitter_count {
        let emitter_offset = fixture_index(emitter_index);
        let x = emitter_offset.mul_add(0.013, -6.0);
        let y = (emitter_offset * 0.071).sin() * 3.0;
        let z = (emitter_offset * 0.047).cos() * 2.0;
        app.world_mut()
            .spawn((RaytracedAudioEmitter3d, Transform::from_xyz(x, y, z)));
    }

    for surface_index in 0..surface_count {
        let column = surface_index % 32;
        let row = surface_index / 32;
        let x = fixture_index(column).mul_add(0.25, -4.0);
        let z = fixture_index(row).mul_add(0.25, -4.0);
        app.world_mut().spawn((
            RaytracedAudioSurface3d::new(
                [
                    Vec3::new(0.0, -0.12, -0.12),
                    Vec3::new(0.0, 0.12, -0.12),
                    Vec3::new(0.0, 0.12, 0.12),
                ],
                AcousticMaterial::default(),
            )?,
            Transform::from_xyz(x, 0.0, z),
        ));
    }
    Ok(app)
}

/// Converts bounded benchmark indices exactly into a meter-scale fixture coordinate.
fn fixture_index(index: usize) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(u16::MAX))
}
