//! Trace direct and first-order reflected paths in a two-dimensional scene.

use bevy_raytraced_audio::{
    AcousticMaterial, AcousticScene2d, Emitter2d, Listener2d, Point2, Segment2d,
};
use std::error::Error;

/// Runs a complete 2D reflection query and prints the direct response and every valid path.
fn main() -> Result<(), Box<dyn Error>> {
    // The core uses meter coordinates and stays independent of Bevy and audio devices.
    let mut scene = AcousticScene2d::default();
    let wall = Segment2d::try_new(
        Point2::try_new(1.0, -1.0)?,
        Point2::try_new(1.0, 3.0)?,
        AcousticMaterial::default(),
    )?;
    scene.add_segment(wall);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(-1.0, 2.0)?);

    // The direct result retains the Bevy adapter's existing aggregate response contract.
    let response = scene.trace(emitter, listener);
    let direct_distance_m = response.direct.distance_m();
    let direct_gain = response.direct.gain();
    println!("direct path: {direct_distance_m} m, gain {direct_gain:?}");

    // Consume paths lazily to inspect exact solver geometry and band energy.
    for path in scene.reflection_paths(emitter, listener) {
        let surface_index = path.surface_index().index();
        let reflection_x = path.reflection_point().x_m();
        let reflection_y = path.reflection_point().y_m();
        let image_x = path.image_source().x_m();
        let image_y = path.image_source().y_m();
        let distance_m = path.distance_m();
        let relative_energy = path.relative_energy();
        println!(
            "surface {surface_index}: point ({reflection_x}, {reflection_y}) m, image ({image_x}, {image_y}) m, path {distance_m} m, energy {relative_energy:?}"
        );
    }
    Ok(())
}
