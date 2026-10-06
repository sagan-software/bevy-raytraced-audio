//! Double-precision positions produced by the three-dimensional acoustic solver.

/// A solver-derived position in three-dimensional space, with coordinates measured in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverPoint3d {
    /// X-coordinate in meters.
    x: f64,
    /// Y-coordinate in meters.
    y: f64,
    /// Z-coordinate in meters.
    z: f64,
}

impl SolverPoint3d {
    /// Returns the X-coordinate in meters.
    #[must_use]
    pub const fn x_m(self) -> f64 {
        self.x
    }

    /// Returns the Y-coordinate in meters.
    #[must_use]
    pub const fn y_m(self) -> f64 {
        self.y
    }

    /// Returns the Z-coordinate in meters.
    #[must_use]
    pub const fn z_m(self) -> f64 {
        self.z
    }

    /// Creates a finite position from double-precision solver geometry.
    pub(crate) const fn new(x_m: f64, y_m: f64, z_m: f64) -> Self {
        Self {
            x: x_m,
            y: y_m,
            z: z_m,
        }
    }
}
