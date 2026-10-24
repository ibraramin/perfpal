// Precision corpus — JS spread accumulation (PERF-JS-ALLOC-003): planted TP.
export function collect(items) {
  let acc = [];
  for (const item of items) {
    acc = [...acc, item];
  }
  return acc;
}
