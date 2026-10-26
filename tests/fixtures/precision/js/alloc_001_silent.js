// Precision corpus — JS literal silent control (must stay silent).
export function build(count) {
  const points = [];
  for (let i = 0; i < count; i++) {
    points.push({ index: i });
  }
  return points;
}
