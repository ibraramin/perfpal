// Precision corpus — JS guarded-recursion silent control (must stay silent).
export function walk(node) {
  if (node.depth <= 0) {
    return 0;
  }
  return walk(node.next);
}
