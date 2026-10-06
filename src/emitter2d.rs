//! A sound source position in the XY propagation plane.

use crate::Point2;

/// A 2D sound source location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emitter2d {
    /// Source position in meters.
    position: Point2,
}

impl Emitter2d {
    /// Creates a source at the supplied XY position.
    #[must_use]
    pub const fn new(position: Point2) -> Self {
        Self { position }
    }

    /// Returns the source position in meters.
    #[must_use]
    pub const fn position(self) -> Point2 {
        self.position
    }
}
