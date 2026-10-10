//! Fixed-length cache counter runs, separate from Criterion's adaptive sampling.
use acoustic_performance::{CACHE_CASES, CacheWorkload};

/// Records identical input sequences and exact invalidation counts for each policy.
fn main() {
    let iterations: u32 = std::env::args()
        .nth(1)
        .map_or(1000, |s| s.parse().expect("positive iterations"));
    assert!(iterations > 0);
    println!("[");
    let mut separator = "";
    for name in CACHE_CASES {
        for enabled in [false, true] {
            let mut workload = CacheWorkload::new(name, enabled);
            let checksum = workload.run(iterations);
            assert!(checksum.is_finite());
            println!(
                "{separator}{{\"name\":\"{name}\",\"cache\":{enabled},\"iterations\":{iterations},\"checksum\":{checksum},\"statistics\":{}}}",
                workload.statistics_json()
            );
            separator = ",";
        }
    }
    println!("]");
}
