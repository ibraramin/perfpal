// PERF-JS-LOOP-007 — linear membership scan inside a loop (Warn).
// True positive: `includes` per iteration. Silent controls: membership
// outside a loop, and the small-collection accepted FP class (< 16 trips).

export function intersect(ids, seen) {
  const hits = [];
  for (const id of ids) {
    if (seen.includes(id)) { // TP PERF-JS-LOOP-007
      hits.push(id);
    }
  }
  return hits;
}

export function containsOnce(ids, seen) {
  return seen.includes(ids[0]); // silent: outside every loop
}

export function smallWindow(ids, seen) {
  const hits = [];
  for (let i = 0; i < 4; i++) {
    if (seen.includes(ids[i])) { // silent: canonical trip count 4 < 16
      hits.push(ids[i]);
    }
  }
  return hits;
}
