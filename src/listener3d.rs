//! A listener position in three-dimensional world space.

use crate::Point3;

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
}
