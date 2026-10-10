//! Bounded deterministic workload driver for Callgrind, heaptrack, perf and Hyperfine.
use acoustic_performance::{CASES, Workload};
/// Profiles a selected case without Criterion's sampling/analysis overhead.
fn main() {
    let mut args = std::env::args().skip(1);
    let name = args
        .next()
        .unwrap_or_else(|| CASES.first().expect("nonempty inventory").to_string());
    let count = args
        .next()
        .map_or(100, |n| n.parse().expect("integer iteration count"));
    let mut workload = Workload::new(&name);
    #[cfg(all(feature = "sampling", target_os = "linux"))]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(997)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("sampling profiler");
    println!("{}", workload.run(count));
    #[cfg(all(feature = "sampling", target_os = "linux"))]
    {
        let report = guard.report().build().expect("profile report");
        report
            .flamegraph(std::fs::File::create("flamegraph.svg").expect("flamegraph file"))
            .expect("flamegraph");
        std::fs::write("profile-stacks.txt", format!("{report:?}")).expect("stack report");
    }
}
