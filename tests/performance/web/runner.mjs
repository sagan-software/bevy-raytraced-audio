import init, {Workload, case_names} from './pkg/acoustic_performance.js';
await init();
const cases = case_names().split('\n');
const status = document.querySelector('#status');
const run = document.querySelector('#run');
run.disabled = false;
status.textContent = `${cases.length} workloads ready`;
window.benchmarkResults = null;
window.runBenchmarks = async function({samples = 30, targetMs = 30} = {}) {
  if (!Number.isInteger(samples) || samples < 10 || samples > 200 || targetMs < 10 || targetMs > 1000) throw Error('Invalid sampling parameters');
  const results = [];
  let hidden = document.hidden;
  const visibility = () => {hidden ||= document.hidden;};
  document.addEventListener('visibilitychange', visibility);
  run.disabled = true;
  document.querySelector('#results').replaceChildren();
  try {
    for (const name of cases) {
      status.textContent = `Measuring ${name}`;
      await new Promise(resolve => setTimeout(resolve, 20));
      const workload = new Workload(name);
      try {
        let iterations = 1;
        let elapsed = 0;
        do {
          const start = performance.now();
          const checksum = workload.run(iterations);
          if (!Number.isFinite(checksum)) throw Error(`Non-finite output: ${name}`);
          elapsed = performance.now() - start;
          if (elapsed < targetMs) iterations *= 2;
        } while (elapsed < targetMs && iterations < 1048576);
        const times = [];
        let checksum = 0;
        for (let i = 0; i < samples; i++) {
          const start = performance.now();
          checksum += workload.run(iterations);
          times.push((performance.now() - start) / iterations);
        }
        if (!Number.isFinite(checksum)) throw Error(`Invalid checksum: ${name}`);
        const sorted = [...times].sort((a,b) => a-b);
        const row = {name, iterations, samples_ms: times, median_ms: sorted[Math.floor(samples/2)], p95_ms: sorted[Math.ceil(samples*.95)-1], checksum};
        results.push(row);
        const tr = document.createElement('tr');
        for (const text of [name, row.median_ms.toFixed(4), row.p95_ms.toFixed(4)]) {
          const td = document.createElement('td'); td.textContent = text; tr.append(td);
        }
        document.querySelector('#results').append(tr);
      } finally { workload.free(); }
    }
    window.benchmarkResults = {build: document.querySelector('#build').value, userAgent: navigator.userAgent, hardwareConcurrency: navigator.hardwareConcurrency, timeOrigin: performance.timeOrigin, timestamp: new Date().toISOString(), hidden, results};
    document.querySelector('#save').disabled = false;
    status.textContent = hidden ? 'Completed, but the tab was hidden: rerun visibly for timing claims.' : 'Complete';
    return window.benchmarkResults;
  } finally { document.removeEventListener('visibilitychange', visibility); run.disabled = false; }
};
run.onclick = () => window.runBenchmarks().catch(error => {status.textContent = String(error);});
document.querySelector('#save').onclick = () => {
  const url = URL.createObjectURL(new Blob([JSON.stringify(window.benchmarkResults,null,2)], {type:'application/json'}));
  const link = document.createElement('a'); link.href = url; link.download = `acoustic-${window.benchmarkResults.build}.json`; link.click(); URL.revokeObjectURL(url);
};
