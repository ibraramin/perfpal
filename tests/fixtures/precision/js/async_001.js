// Precision corpus — JS await in a loop (PERF-JS-ASYNC-001): planted TP.
async function fetchJson(url) {
  return url.length;
}

export async function loadAll(urls) {
  const payloads = [];
  for (const url of urls) {
    payloads.push(await fetchJson(url));
  }
  return payloads;
}
