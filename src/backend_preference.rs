//! Public backend preferences shared by the dimension-specific Bevy adapters.

/// Selects how a dimension-specific Bevy adapter computes acoustic propagation.
///
/// Both choices keep the existing Bevy audio playback system in control of
/// playback. The current release has no GPU backend, so each choice uses CPU
/// tracing; [`Auto`](Self::Auto) also logs that GPU compute is unavailable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioBackendPreference {
    /// Always use the CPU tracer for acoustic propagation.
    ///
    /// This is the default. It works without `bevy_render` and does not require
    /// a GPU device or render sub-app.
    #[default]
    Cpu,
    /// Prefer optional GPU compute after its render device and pipeline are ready.
    ///
    /// Keep CPU tracing active while GPU setup is pending or after GPU failure.
    ///
    /// The current release has no GPU backend, so this preference currently uses CPU.
    Auto,
}
