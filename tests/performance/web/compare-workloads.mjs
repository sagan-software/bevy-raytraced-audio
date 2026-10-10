// Import this helper in the browser console with two preserved build directories.
export async function compareWorkloads(before, after, prefix = "") {
  const modules = await Promise.all([before, after].map(path => import(`${path}/pkg/acoustic_performance.js`)));
  await Promise.all(modules.map(module => module.default()));
  if (modules[0].case_names() !== modules[1].case_names()) throw Error("Workload inventory changed");
  const cases = modules[0].case_names().split("\n").filter(name => name.startsWith(prefix));
  if (!cases.length) throw Error("Empty workload comparison");
  const results = [];
  let hidden = document.hidden;
  const visibility = () => {
    hidden ||= document.hidden;
  };
  document.addEventListener("visibilitychange", visibility);
  try {
    for (const name of cases) {
      document.querySelector("#status").textContent = `Workload comparison: ${name}`;
      await new Promise(resolve => {
        const channel = new MessageChannel();
        channel.port1.onmessage = () => {
          channel.port1.close();
          channel.port2.close();
          resolve();
        };
        channel.port2.postMessage(null);
      });
      const calibration = new modules[0].Workload(name);
      let iterations = 1;
      try {
        while (iterations < 1048576) {
          const start = performance.now();
          if (!Number.isFinite(calibration.run(iterations))) throw Error("Invalid workload output");
          if (performance.now() - start >= 30) break;
          iterations *= 2;
        }
      } finally {
        calibration.free();
      }
      const workloads = modules.map(module => new module.Workload(name));
      const times = [[], []];
      try {
        // Equal warmup and operation counts keep moving-source coefficient states aligned.
        for (const workload of workloads) workload.run(iterations);
        for (let sample = 0; sample < 30; sample++) {
          const checksums = [];
          for (const mode of sample % 2 ? [1, 0] : [0, 1]) {
            const start = performance.now();
            checksums[mode] = workloads[mode].run(iterations);
            times[mode].push((performance.now() - start) / iterations);
          }
          if (!Number.isFinite(checksums[0]) || checksums[0] !== checksums[1]) {
            throw Error(`Workload output changed: ${name}`);
          }
        }
        results.push({ name, iterations, before_ms: times[0], after_ms: times[1] });
      } finally {
        for (const workload of workloads) workload.free();
      }
    }
    return {
      schema: 1,
      before,
      after,
      hidden,
      userAgent: navigator.userAgent,
      hardwareConcurrency: navigator.hardwareConcurrency,
      measurement_kind: "synchronous_wasm",
      sequence_alignment: "identical-batches",
      results,
    };
  } finally {
    document.removeEventListener("visibilitychange", visibility);
  }
}
