// Precision corpus — JS membership scan (PERF-JS-LOOP-007): planted TP.
export function intersect(ids, seen) {
  const hits = [];
  for (const id of ids) {
    if (seen.includes(id)) {
      hits.push(id);
    }
  }
  return hits;
}
