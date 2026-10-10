//! Paired cache policies execute exactly the same deterministic input sequences.
use acoustic_performance::{CACHE_CASES, CacheWorkload};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

/// Measures cache hits, misses, comparisons, key maintenance and actual solver work together.
fn cache_behavior(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_behavior");
    let reverse = std::env::var_os("CACHE_BENCH_REVERSE").is_some();
    let modes = if reverse {
        [("on", true), ("off", false)]
    } else {
        [("off", false), ("on", true)]
    };
    for case in CACHE_CASES {
        for (label, enabled) in modes {
            let mut workload = CacheWorkload::new(case, enabled);
            group.bench_function(BenchmarkId::new(*case, label), |b| {
                b.iter(|| std::hint::black_box(workload.run(1)));
            });
        }
    }
    group.finish();
}
criterion_group!(benches, cache_behavior);
criterion_main!(benches);
