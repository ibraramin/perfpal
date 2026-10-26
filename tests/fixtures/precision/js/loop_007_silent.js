// Precision corpus — JS small-collection silent control (must stay silent).
export function small(ids, seen) {
  const hits = [];
  for (let i = 0; i < 4; i++) {
    if (seen.includes(ids[i])) {
      hits.push(ids[i]);
    }
  }
  return hits;
}
