//! Compare a source enclosed in a box with and without a source clearance radius.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene3d, Emitter3d, Listener3d, Point3, Triangle3d,
};
use std::error::Error;

/// Builds a closed 1 m cube around the origin, traces a listener outside it, and prints both results.
fn main() -> Result<(), Box<dyn Error>> {
    let mut scene = AcousticScene3d::default();
    // Each cube face is two opaque triangles; the default material transmits nothing.
    // A face's corners are its two in-plane coordinates placed around the fixed one.
    let place = [
        |side: f32, u: f32, v: f32| Point3::try_new(side, u, v),
        |side: f32, u: f32, v: f32| Point3::try_new(u, side, v),
        |side: f32, u: f32, v: f32| Point3::try_new(u, v, side),
    ];
    for corner in place {
        for side in [-0.5_f32, 0.5] {
            let [a, b, c, d] = [
                corner(side, -0.5, -0.5)?,
                corner(side, 0.5, -0.5)?,
                corner(side, 0.5, 0.5)?,
                corner(side, -0.5, 0.5)?,
            ];
            scene.add_triangle(Triangle3d::try_new([a, b, c], AcousticMaterial::default())?);
            scene.add_triangle(Triangle3d::try_new([a, c, d], AcousticMaterial::default())?);
        }
    }
    let listener = Listener3d::new(Point3::try_new(4.0, 0.0, 0.0)?);
    let enclosed = Emitter3d::new(Point3::try_new(0.0, 0.0, 0.0)?);
    // The cube corners lie about 0.87 m from its centre, so 1 m clears every face.
    let cleared = enclosed.try_with_clearance(1.0)?;

    for (label, emitter) in [
        ("without clearance", enclosed),
        ("with 1 m clearance", cleared),
    ] {
        let direct = scene.trace(emitter, listener).direct;
        let occluded = direct.is_occluded();
        let gain = direct.gain();
        println!("{label}: occluded {occluded}, gain {gain:?}");
    }
    Ok(())
}
