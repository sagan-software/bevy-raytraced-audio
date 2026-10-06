//! Explicit local-space acoustic geometry for the 3d adapter.

use bevy::prelude::{Component, Vec3};
use bevy_raytraced_audio::{AcousticMaterial, GeometryError, Point3, Triangle3d};

/// A local-space triangle included in the 3D acoustic scene.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaytracedAudioSurface3d {
    /// Validated local-space triangle and material.
    pub(super) triangle: Triangle3d,
}

impl RaytracedAudioSurface3d {
    /// Creates a local-space triangle from finite, non-collinear vertices.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite vertex or
    /// [`GeometryError::DegeneratePrimitive`] when the vertices are collinear.
    pub fn new(vertices: [Vec3; 3], material: AcousticMaterial) -> Result<Self, GeometryError> {
        let vertices = [
            Point3::try_new(vertices[0].x, vertices[0].y, vertices[0].z)?,
            Point3::try_new(vertices[1].x, vertices[1].y, vertices[1].z)?,
            Point3::try_new(vertices[2].x, vertices[2].y, vertices[2].z)?,
        ];
        let triangle = Triangle3d::try_new(vertices, material)?;
        Ok(Self { triangle })
    }

    /// Returns the local-space triangle used by the scene adapter.
    #[must_use]
    pub const fn triangle(self) -> Triangle3d {
        self.triangle
    }
}
