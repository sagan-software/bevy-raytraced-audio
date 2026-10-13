//! A listener position in three-dimensional world space.

use crate::Point3;
use crate::math3d::Vector3;
use crate::ray_trace::within_coherent_motion;

/// A 3D listener location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener3d {
    /// Listener position in meters.
    position: Point3,
}

impl Listener3d {
    /// Creates a listener at the supplied XYZ position.
    #[must_use]
    pub const fn new(position: Point3) -> Self {
        Self { position }
    }

    /// Returns the listener position in meters.
    #[must_use]
    pub const fn position(self) -> Point3 {
        self.position
    }

    /// Returns whether `other` is at most [`COHERENT_TRACE_MOTION_M`](crate::COHERENT_TRACE_MOTION_M) meters away.
    ///
    /// A coherent listener trace keeps casting from its start pose while this stays true.
    #[must_use]
    pub fn within_trace_motion(self, other: Self) -> bool {
        let delta =
            Vector3::from_point(other.position).subtract(Vector3::from_point(self.position));
        within_coherent_motion(delta.dot(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::Listener3d;
    use crate::Point3;

    /// One meter stays coherent. The next representable step does not.
    #[test]
    fn coherent_motion_includes_one_meter() {
        let origin = Listener3d::new(Point3::try_new(0.0, 0.0, 0.0).expect("finite"));
        let on_radius = Listener3d::new(Point3::try_new(1.0, 0.0, 0.0).expect("finite"));
        let past_radius = Listener3d::new(
            Point3::try_new(f32::from_bits(1.0_f32.to_bits() + 1), 0.0, 0.0).expect("finite"),
        );
        let diagonal = Listener3d::new(Point3::try_new(1.0, 1.0, 0.0).expect("finite"));
        assert!(origin.within_trace_motion(origin));
        assert!(origin.within_trace_motion(on_radius));
        assert!(!origin.within_trace_motion(past_radius));
        assert!(!origin.within_trace_motion(diagonal));
    }
}
