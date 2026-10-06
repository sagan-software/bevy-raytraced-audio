//! A triangle with explicit acoustic material properties.

use crate::math3d::Vector3;
use crate::{AcousticMaterial, GeometryError, Point3};

/// A non-degenerate 3D acoustic surface triangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle3d {
    /// Triangle vertices in meters and counter-clockwise winding.
    vertices: [Point3; 3],
    /// Frequency-dependent absorption at this surface.
    material: AcousticMaterial,
}

impl Triangle3d {
    /// Creates a triangle and rejects zero-area vertices.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::DegeneratePrimitive`] when the vertices are collinear.
    pub fn try_new(
        vertices: [Point3; 3],
        material: AcousticMaterial,
    ) -> Result<Self, GeometryError> {
        let first = Vector3::from_point(vertices[0]);
        let second = Vector3::from_point(vertices[1]);
        let third = Vector3::from_point(vertices[2]);
        let edge_a = second.subtract(first);
        let edge_b = third.subtract(first);

        if edge_a.cross(edge_b).length() == 0.0 {
            return Err(GeometryError::DegeneratePrimitive);
        }

        Ok(Self { vertices, material })
    }

    /// Returns the triangle vertices in meters.
    #[must_use]
    pub const fn vertices(self) -> [Point3; 3] {
        self.vertices
    }

    /// Returns this triangle's acoustic material.
    #[must_use]
    pub const fn material(self) -> AcousticMaterial {
        self.material
    }
}
