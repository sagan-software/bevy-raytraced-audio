import init, { cache_case_names, CacheWorkload } from "./pkg/acoustic_performance.js";
await init();
const build = await fetch("./build.json").then(response => response.json());
// Yield between cases without background-tab timer clamping inside paired samples.
const yieldToBrowser = () =>
  new Promise(resolve => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      channel.port2.close();
      resolve();
    };
    channel.port2.postMessage(null);
  });
// Alternate policy order across samples to reduce drift and thermal-order bias.
window.runCacheBenchmarks = async function({ samples = 30, targetMs = 30, counterFrames = 1000 } = {}) {
  if (!Number.isInteger(samples) || samples < 10 || samples > 200 || targetMs < 10 || targetMs > 1000) {
    throw Error("Invalid sampling parameters");
  }
  const results = [];
  let hidden = document.hidden;
  const visibility = () => {
    hidden ||= document.hidden;
  };
  document.addEventListener("visibilitychange", visibility);
  try {
    for (const name of cache_case_names().split("\n")) {
      document.querySelector("#status").textContent = `Cache comparison: ${name}`;
      await yieldToBrowser();
      const workloads = [new CacheWorkload(name, false), new CacheWorkload(name, true)];
      try {
        const iterations = workloads.map(workload => {
          let count = 1;
          while (count < 1048576) {
            const start = performance.now();
            if (!Number.isFinite(workload.run(count))) throw Error("Invalid acoustic result");
            if (performance.now() - start >= targetMs) break;
            count *= 2;
          }
          return count;
        });
        const times = [[], []];
        // Each mixed-idle batch must contain a miss: all-hit batches can round to zero
        // under the browser's coarse timer. Both policies still execute identical frames.
        const commonIterations = Math.max(iterations[0], name.endsWith("mixed_idle") ? 10 : 1);
        for (const mode of [0, 1]) {
          if (!name.endsWith("static")) iterations[mode] = commonIterations;
          workloads[mode].free();
          workloads[mode] = new CacheWorkload(name, !!mode);
          workloads[mode].run(1);
        }
        for (let sample = 0; sample < samples; sample++) {
          const checksums = [];
          for (const mode of sample % 2 ? [1, 0] : [0, 1]) {
            const start = performance.now();
            const checksum = workloads[mode].run(iterations[mode]);
            const elapsed = performance.now() - start;
            if (!Number.isFinite(checksum)) throw Error("Invalid acoustic result");
            checksums[mode] = checksum;
            times[mode].push(elapsed / iterations[mode]);
          }
          if (!name.endsWith("static") && checksums[0] !== checksums[1]) {
            throw Error(`Mismatched paired input sequence: ${name}`);
          }
        }
        for (const mode of [0, 1]) {
          const counters = new CacheWorkload(name, !!mode);
          try {
            const checksum = counters.run(counterFrames);
            results.push({
              name,
              cache: !!mode,
              iterations: iterations[mode],
              samples_ms: times[mode],
              counterFrames,
              checksum,
              statistics: JSON.parse(counters.statistics_json()),
            });
          } finally {
            counters.free();
          }
        }
      } finally {
        for (const workload of workloads) workload.free();
      }
    }
    return window.cacheBenchmarkResults = {
      schema: 1,
      scheduling: "yield-between-cases",
      sequence_alignment: "common-batches-reset-after-calibration",
      build,
      timestamp: new Date().toISOString(),
      hidden,
      userAgent: navigator.userAgent,
      hardwareConcurrency: navigator.hardwareConcurrency,
      results,
    };
  } finally {
    document.removeEventListener("visibilitychange", visibility);
  }
};

const run = document.querySelector("#cache-run");
run.disabled = false;
run.onclick = async () => {
  run.disabled = true;
  try {
    await window.runCacheBenchmarks();
    document.querySelector("#status").textContent = window.cacheBenchmarkResults.hidden
      ? "Completed with a hidden tab; rerun visibly for timing claims."
      : "Cache comparison complete";
    document.querySelector("#cache-save").disabled = false;
  } catch (error) {
    document.querySelector("#status").textContent = String(error);
  } finally {
    run.disabled = false;
  }
};
document.querySelector("#cache-save").onclick = () => {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(window.cacheBenchmarkResults, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = "cache-comparison.json";
  link.click();
  URL.revokeObjectURL(url);
};
