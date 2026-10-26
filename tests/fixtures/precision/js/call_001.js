// Precision corpus — JS unguarded recursion (PERF-JS-CALL-001): planted TP.
export function walk(node) {
  return walk(node.next);
}
