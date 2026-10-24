// Precision corpus — JS loop-invariant literal (PERF-JS-ALLOC-001): planted TP.
export function build(count) {
  const frames = [];
  for (let i = 0; i < count; i++) {
    frames.push({ status: "new" });
  }
  return frames;
}
