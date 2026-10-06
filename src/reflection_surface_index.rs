//! Scene-local identifiers for registered acoustic surfaces.

/// Identifies a surface by its zero-based position in one acoustic scene's insertion order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ReflectionSurfaceIndex {
    /// Zero-based insertion position in the scene that produced a reflection path.
    index: usize,
}

impl ReflectionSurfaceIndex {
    /// Returns the zero-based position of this surface in the queried acoustic scene.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }

    /// Creates a scene-local surface index from its insertion position.
    pub(crate) const fn new(index: usize) -> Self {
        Self { index }
    }
}
