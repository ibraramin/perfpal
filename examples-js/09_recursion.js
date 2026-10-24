// PERF-JS-CALL-001 — unguarded direct recursion (Info). True positive: no
// comparison precedes the self-call. Silent control: a visible base-case
// comparison guards the recursion.

export function descend(node) {
  return descend(node.next); // TP PERF-JS-CALL-001
}

export function countdown(n) {
  if (n <= 0) {
    return 0;
  }
  return countdown(n - 1); // silent: base-case comparison precedes the call
}
