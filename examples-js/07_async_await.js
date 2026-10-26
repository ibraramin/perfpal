// PERF-JS-ASYNC-001 — `await` inside a loop (Warn). True positive: an
// awaited fetch per iteration. Silent control: the same await outside every
// loop, and a non-async helper with no await events.
async function fetchJson(url) {
  return url.length;
}

export async function loadAll(urls) {
  const payloads = [];
  for (const url of urls) {
    payloads.push(await fetchJson(url)); // TP PERF-JS-ASYNC-001
  }
  return payloads;
}

export async function loadOne(url) {
  const payload = await fetchJson(url); // silent: outside every loop
  return payload;
}
