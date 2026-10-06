//! A line segment with explicit acoustic material properties.

use crate::{AcousticMaterial, GeometryError, Point2};

/// A non-degenerate 2D acoustic surface segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment2d {
    /// First endpoint in meters.
    start: Point2,
    /// Second endpoint in meters.
    end: Point2,
    /// Frequency-dependent absorption at this surface.
    material: AcousticMaterial,
}

impl Segment2d {
    /// Creates a segment and rejects coincident endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::DegeneratePrimitive`] when both endpoints are equal.
    pub fn try_new(
        start: Point2,
        end: Point2,
        material: AcousticMaterial,
    ) -> Result<Self, GeometryError> {
        if start == end {
            return Err(GeometryError::DegeneratePrimitive);
        }

        Ok(Self {
            start,
            end,
            material,
        })
    }

    /// Returns the first endpoint in meters.
    #[must_use]
    pub const fn start(self) -> Point2 {
        self.start
    }

    /// Returns the second endpoint in meters.
    #[must_use]
    pub const fn end(self) -> Point2 {
        self.end
    }

    /// Returns this segment's acoustic material.
    #[must_use]
    pub const fn material(self) -> AcousticMaterial {
        self.material
    }
}
