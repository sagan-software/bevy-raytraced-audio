//! A listener position in the XY propagation plane.

use crate::Point2;

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
}
