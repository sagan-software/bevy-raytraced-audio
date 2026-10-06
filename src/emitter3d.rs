//! A sound source position in three-dimensional world space.

use crate::Point3;

/// A 3D sound source location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emitter3d {
    /// Source position in meters.
    position: Point3,
}

impl Emitter3d {
    /// Creates a source at the supplied XYZ position.
    #[must_use]
    pub const fn new(position: Point3) -> Self {
        Self { position }
    }

    /// Returns the source position in meters.
    #[must_use]
    pub const fn position(self) -> Point3 {
        self.position
    }
}
