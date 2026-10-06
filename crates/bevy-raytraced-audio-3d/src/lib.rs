//! Public 3D adapter facade with version-selected Bevy integration.

#[cfg(feature = "bevy_0_17")]
pub mod bevy_0_17 {
    //! Public integration types for Bevy 0.17.
    pub use compat_0_17::audio_3d::*;
}

#[cfg(feature = "bevy_0_18")]
pub mod bevy_0_18 {
    //! Public integration types for Bevy 0.18.
    pub use compat_0_18::audio_3d::*;
}

#[cfg(feature = "bevy_0_19")]
pub mod bevy_0_19 {
    //! Public integration types for Bevy 0.19.
    pub use compat_0_19::audio_3d::*;
}

#[cfg(feature = "bevy_0_20")]
pub mod bevy_0_20 {
    //! Public integration types for Bevy 0.20.
    pub use compat_0_20::audio_3d::*;
}

#[cfg(all(
    not(feature = "bevy_0_20"),
    not(feature = "bevy_0_19"),
    not(feature = "bevy_0_18"),
    feature = "bevy_0_17"
))]
pub use bevy_0_17::*;
#[cfg(all(
    not(feature = "bevy_0_20"),
    not(feature = "bevy_0_19"),
    feature = "bevy_0_18"
))]
pub use bevy_0_18::*;
#[cfg(all(not(feature = "bevy_0_20"), feature = "bevy_0_19"))]
pub use bevy_0_19::*;
#[cfg(feature = "bevy_0_20")]
pub use bevy_0_20::*;
