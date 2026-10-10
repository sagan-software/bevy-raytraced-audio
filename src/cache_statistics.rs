//! Observable result-cache decisions, independent of the cached acoustic values.

/// Counters accumulated by one listener-trace output across calls.
///
/// A miss records the first differing input, in scene/listener/settings/sources order.
/// Simultaneous changes therefore count as one primary invalidation reason. Disabling result
/// reuse counts a forced computation, without spending time comparing cache keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TraceCacheStatistics {
    /// Requests served entirely from the previous completed trace.
    pub hits: u64,
    /// Requests that execute the acoustic solver, including first and forced calls.
    pub computations: u64,
    /// Computations explicitly requested with result reuse disabled.
    pub forced: u64,
    /// Computations with no completed trace available.
    pub cold: u64,
    /// Computations triggered by a different scene revision.
    pub scene_changes: u64,
    /// Computations triggered by a different listener position.
    pub listener_changes: u64,
    /// Computations triggered by different tracing settings.
    pub settings_changes: u64,
    /// Computations triggered by source movement, count, or ordering changes.
    pub source_changes: u64,
}

/// Primary reason a completed listener result cannot be reused.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TraceCacheMiss {
    /// Explicitly disabled reuse.
    Forced,
    /// No completed result.
    Cold,
    /// A changed scene revision.
    Scene,
    /// A changed listener position.
    Listener,
    /// Changed solver settings.
    Settings,
    /// Changed source positions or inventory.
    Sources,
}

impl TraceCacheStatistics {
    /// Counts one request without changing its acoustic result.
    pub(crate) const fn record(&mut self, miss: Option<TraceCacheMiss>) {
        let Some(miss) = miss else {
            self.hits = self.hits.saturating_add(1);
            return;
        };
        self.computations = self.computations.saturating_add(1);
        let counter = match miss {
            TraceCacheMiss::Forced => &mut self.forced,
            TraceCacheMiss::Cold => &mut self.cold,
            TraceCacheMiss::Scene => &mut self.scene_changes,
            TraceCacheMiss::Listener => &mut self.listener_changes,
            TraceCacheMiss::Settings => &mut self.settings_changes,
            TraceCacheMiss::Sources => &mut self.source_changes,
        };
        *counter = counter.saturating_add(1);
    }
}

/// Scheduling decisions made by a Bevy acoustic adapter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TraceScheduleStatistics {
    /// Adapter updates that scheduled a listener-trace attempt.
    pub scheduled: u64,
    /// Updates whose inputs were unchanged after the configured interval elapsed.
    pub unchanged: u64,
    /// Updates deferred before the configured trace interval elapsed.
    pub throttled: u64,
    /// Refreshes of scene geometry from Bevy entities, before lazy BVH construction.
    pub scene_refreshes: u64,
}
