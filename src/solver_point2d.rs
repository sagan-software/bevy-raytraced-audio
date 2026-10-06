//! Double-precision positions produced by the two-dimensional acoustic solver.

/// A solver-derived position in the XY plane, with coordinates measured in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverPoint2d {
    /// X-coordinate in meters.
    x: f64,
    /// Y-coordinate in meters.
    y: f64,
}

impl SolverPoint2d {
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

    /// Creates a finite position from double-precision solver geometry.
    pub(crate) const fn new(x_m: f64, y_m: f64) -> Self {
        Self { x: x_m, y: y_m }
    }
}
