// PERF-JS-ASYNC-001 — `await` inside a loop (Warn). True positive: an
// awaited fetch per iteration. Silent control: the same await outside every
// loop. (`fetchJson` returns a plain number, so no other rule can fire.)

async function fetchJson(url: string): Promise<number> {
  return url.length;
}

export async function loadAll(urls: readonly string[]): Promise<number[]> {
  const payloads: number[] = [];
  for (const url of urls) {
    payloads.push(await fetchJson(url)); // TP PERF-JS-ASYNC-001
  }
  return payloads;
}

export async function loadOne(url: string): Promise<number> {
  const payload = await fetchJson(url); // silent: outside every loop
  return payload;
}
