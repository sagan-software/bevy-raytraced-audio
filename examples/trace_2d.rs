//! Trace the direct acoustic path between two points in an empty 2D scene.

use bevy_raytraced_audio::{AcousticScene2d, Emitter2d, Listener2d, Point2};
use std::error::Error;

/// Runs the empty-scene propagation example and prints its direct-path gain.
fn main() -> Result<(), Box<dyn Error>> {
    // The core uses world coordinates in meters and does not depend on Bevy.
    let scene = AcousticScene2d::default();
    let emitter = Emitter2d::new(Point2::try_new(-2.0, 0.0)?);
    let listener = Listener2d::new(Point2::try_new(2.0, 0.0)?);

    // The returned distance and transmission bands are ready for an adapter or DSP node.
    let response = scene.trace(emitter, listener);
    println!(
        "direct path: {} m, gain {:?}",
        response.direct.distance_m(),
        response.direct.gain()
    );
    Ok(())
}
