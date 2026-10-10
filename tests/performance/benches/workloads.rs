//! Criterion entry point for the shared native/browser acoustic workload inventory.
use acoustic_performance::{CASES, Workload};
use criterion::Criterion;
use std::hint::black_box;
/// Runs the unchanged inventory with CLI-controlled sampling and named baselines.
fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    for name in CASES {
        let mut workload = Workload::new(name);
        criterion.bench_function(name, |b| b.iter(|| black_box(workload.run(1))));
    }
    criterion.final_summary();
}
