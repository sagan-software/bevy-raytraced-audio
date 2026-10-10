//! Bevy-independent acoustic propagation primitives shared by the 2D and 3D adapters.
//!
//! [`AcousticScene2d`] and [`AcousticScene3d`] trace direct paths, image-source reflections,
//! and listener rays through explicit segments and triangles. Listener traces estimate
//! muffling, room reverb, and outdoor ambience; [`AcousticDspProcessor`] applies smoothed
//! filtering and reverb to decoded samples. `AcousticMaterial`, `BandAbsorption`,
//! and `BandGain` validate per-band energy absorption and amplitude transmission.
//! The Bevy adapters consume response values without replacing Bevy's audio
//! playback system. This crate has no Bevy dependency, so each adapter can target
//! its own supported Bevy release.

mod acoustic_material;
mod backend_preference;
mod band_absorption;
mod band_energy;
mod band_gain;
mod binaural;
mod bvh;
mod dsp;
mod emitter2d;
mod emitter3d;
mod geometry_error;
mod listener2d;
mod listener3d;
mod math2d;
mod math3d;
mod point2;
mod point3;
mod ray_trace;
mod ray_tracer2d;
mod ray_tracer3d;
mod reflection_path2d;
mod reflection_path3d;
mod reflection_surface_index;
mod response;
mod scene2d;
mod scene3d;
mod segment2d;
mod solver_point2d;
mod solver_point3d;
mod triangle3d;

pub use self::acoustic_material::AcousticMaterial;
pub use self::backend_preference::AudioBackendPreference;
pub use self::band_absorption::BandAbsorption;
pub use self::band_energy::BandEnergy;
pub use self::band_gain::BandGain;
pub use self::binaural::{BinauralParams, BinauralProcessor};
pub use self::dsp::{AcousticDspParams, AcousticDspProcessor};
pub use self::emitter2d::Emitter2d;
pub use self::emitter3d::Emitter3d;
pub use self::geometry_error::GeometryError;
pub use self::listener2d::Listener2d;
pub use self::listener3d::Listener3d;
pub use self::point2::Point2;
pub use self::point3::Point3;
pub use self::ray_trace::{
    MuffleFilter, RayKind, RayTraceSettings, ReverbEstimate, SPEED_OF_SOUND_M_PER_S,
    SourceRayResponse, muffle_strength,
};
pub use self::ray_tracer2d::{ListenerTrace2d, RaySegment2d};
pub use self::ray_tracer3d::{ListenerTrace3d, RaySegment3d};
pub use self::reflection_path2d::ReflectionPath2d;
pub use self::reflection_path3d::ReflectionPath3d;
pub use self::reflection_surface_index::ReflectionSurfaceIndex;
pub use self::response::{AcousticResponse, PathResponse};
pub use self::scene2d::AcousticScene2d;
pub use self::scene3d::AcousticScene3d;
pub use self::segment2d::Segment2d;
pub use self::solver_point2d::SolverPoint2d;
pub use self::solver_point3d::SolverPoint3d;
pub use self::triangle3d::Triangle3d;
