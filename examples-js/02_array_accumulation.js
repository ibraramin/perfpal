// PERF-JS-ALLOC-003 — spread accumulation in a loop (Info).
// True positive: the accumulator is reassigned in the loop. Silent controls:
// a snapshot whose receiver is never reassigned, and `concat` outside a loop.

export function flatten(items) {
  let acc = [];
  for (const item of items) {
    acc = [...acc, item]; // TP PERF-JS-ALLOC-003
  }
  return acc;
}

export function snapshotRows(rows) {
  const copies = [];
  for (const row of rows) {
    const view = rows.slice(); // silent: receiver is not reassigned
    copies.push(view);
  }
  return copies;
}

export function merge(a, b) {
  return a.concat(b); // silent: outside every loop
}
