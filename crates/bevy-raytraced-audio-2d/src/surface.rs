//! Explicit local-space acoustic geometry for the 2d adapter.

use bevy::prelude::{Component, Vec2};
use bevy_raytraced_audio::{AcousticMaterial, GeometryError, Point2, Segment2d};

/// A local-space wall segment included in the 2D acoustic scene.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaytracedAudioSurface2d {
    /// Validated local-space line segment and material.
    pub(super) segment: Segment2d,
}

impl RaytracedAudioSurface2d {
    /// Creates a local-space wall from finite, non-coincident endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite endpoint or
    /// [`GeometryError::DegeneratePrimitive`] when both endpoints coincide.
    pub fn new(start: Vec2, end: Vec2, material: AcousticMaterial) -> Result<Self, GeometryError> {
        let start = Point2::try_new(start.x, start.y)?;
        let end = Point2::try_new(end.x, end.y)?;
        let segment = Segment2d::try_new(start, end, material)?;
        Ok(Self { segment })
    }

    /// Returns the local-space segment used by the scene adapter.
    #[must_use]
    pub const fn segment(self) -> Segment2d {
        self.segment
    }
}
