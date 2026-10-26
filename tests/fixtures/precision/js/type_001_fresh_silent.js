// Precision corpus — JS delete-from-fresh-object silent control (must stay silent).
// `merged` is bound to a per-call object literal, so deleting from it cannot
// pollute a long-lived hidden class; PERF-JS-TYPE-001 must not fire.
export function sanitize(config) {
  const merged = { ...config };
  delete merged.unsafeField;
  return merged;
}
