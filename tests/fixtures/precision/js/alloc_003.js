// Precision corpus — JS concat accumulation (PERF-JS-ALLOC-003): planted TP.
export function flatten(items) {
  let acc = [];
  for (const item of items) {
    acc = acc.concat(item);
  }
  return acc;
}
