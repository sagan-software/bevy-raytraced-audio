//! A listener position in the XY propagation plane.

use crate::Point2;
use crate::math2d::Vector2;
use crate::ray_trace::within_coherent_motion;

/// A 2D listener location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener2d {
    /// Listener position in meters.
    position: Point2,
}

impl Listener2d {
    /// Creates a listener at the supplied XY position.
    #[must_use]
    pub const fn new(position: Point2) -> Self {
        Self { position }
    }

    /// Returns the listener position in meters.
    #[must_use]
    pub const fn position(self) -> Point2 {
        self.position
    }

    /// Returns whether `other` is at most [`COHERENT_TRACE_MOTION_M`](crate::COHERENT_TRACE_MOTION_M) meters away.
    ///
    /// A coherent listener trace keeps casting from its start pose while this stays true.
    #[must_use]
    pub fn within_trace_motion(self, other: Self) -> bool {
        let delta =
            Vector2::from_point(other.position).subtract(Vector2::from_point(self.position));
        within_coherent_motion(delta.dot(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::Listener2d;
    use crate::Point2;

    /// One meter stays coherent. The next representable step does not.
    #[test]
    fn coherent_motion_includes_one_meter() {
        let origin = Listener2d::new(Point2::try_new(0.0, 0.0).expect("finite"));
        let on_radius = Listener2d::new(Point2::try_new(1.0, 0.0).expect("finite"));
        let past_radius = Listener2d::new(
            Point2::try_new(f32::from_bits(1.0_f32.to_bits() + 1), 0.0).expect("finite"),
        );
        let diagonal = Listener2d::new(Point2::try_new(1.0, 1.0).expect("finite"));
        assert!(origin.within_trace_motion(origin));
        assert!(origin.within_trace_motion(on_radius));
        assert!(!origin.within_trace_motion(past_radius));
        assert!(!origin.within_trace_motion(diagonal));
    }
}
