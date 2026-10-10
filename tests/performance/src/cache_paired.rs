//! Interleaved native cache timings supplement Criterion's separate policy measurements.
use acoustic_performance::{CACHE_CASES, CacheWorkload};
use std::{hint::black_box, time::Instant};

/// One policy's state and unaggregated timings.
struct Policy {
    /// Stateful deterministic scene, including all timed mutations.
    workload: CacheWorkload,
    /// Adaptive batch length, fixed for every sample of this policy.
    iterations: u32,
    /// Mean milliseconds per operation in each batch.
    samples: Vec<f64>,
    /// Last timed batch's checksum, compared between policies for dynamic scenes.
    checksum: f64,
}

impl Policy {
    /// Warm the scene and choose a batch lasting at least 30 milliseconds.
    fn new(name: &str, enabled: bool) -> Self {
        let mut workload = CacheWorkload::new(name, enabled);
        let mut iterations = 1;
        loop {
            let start = Instant::now();
            let checksum = black_box(workload.run(iterations));
            let elapsed = start.elapsed();
            assert!(checksum.is_finite());
            if elapsed.as_secs_f64() >= 0.03 || iterations >= 1_048_576 {
                break;
            }
            iterations *= 2;
        }
        Self {
            workload,
            iterations,
            samples: Vec::with_capacity(30),
            checksum: 0.0,
        }
    }

    /// Includes mutations, key comparisons, invalidation and full tracing in the timer.
    fn measure(&mut self) {
        let start = Instant::now();
        let checksum = black_box(self.workload.run(self.iterations));
        let elapsed = start.elapsed().as_secs_f64() * 1000.0 / f64::from(self.iterations);
        assert!(checksum.is_finite() && elapsed.is_finite() && elapsed > 0.0);
        self.samples.push(elapsed);
        self.checksum = checksum;
    }
}

/// Alternate policy order in every sample to reduce drift between entire Criterion cases.
fn main() {
    println!(
        "{{\"schema\":1,\"platform\":\"{}-{}\",\"scheduling\":\"interleaved-policies\",\"sequence_alignment\":\"common-batches-reset-after-calibration\",\"hidden\":false,\"results\":[",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let mut separator = "";
    for name in CACHE_CASES {
        eprintln!("Measuring {name}");
        let mut policies = [false, true].map(|enabled| Policy::new(name, enabled));
        let common_iterations = policies.first().expect("two policies").iterations;
        for (enabled, policy) in [false, true].into_iter().zip(&mut policies) {
            // Static scenes never move, so their much faster cache hits may use larger batches.
            if !name.ends_with("static") {
                policy.iterations = common_iterations;
            }
            policy.workload = CacheWorkload::new(name, enabled);
            black_box(policy.workload.run(1));
        }
        for sample in 0..30 {
            if sample % 2 == 0 {
                policies.iter_mut().for_each(Policy::measure);
            } else {
                policies.iter_mut().rev().for_each(Policy::measure);
            }
            if !name.ends_with("static") {
                let [off, on] = &policies;
                assert_eq!(off.checksum, on.checksum, "paired input sequence: {name}");
            }
        }
        for (enabled, policy) in [false, true].into_iter().zip(policies) {
            let mut diagnostics = CacheWorkload::new(name, enabled);
            let checksum = diagnostics.run(100);
            assert!(checksum.is_finite());
            println!(
                "{separator}{{\"name\":\"{name}\",\"cache\":{enabled},\"iterations\":{},\"samples_ms\":{:?},\"counterFrames\":100,\"checksum\":{checksum},\"statistics\":{}}}",
                policy.iterations,
                policy.samples,
                diagnostics.statistics_json()
            );
            separator = ",";
        }
    }
    println!("]}}");
}
