// Precision corpus — JS depth-1 silent control (must stay silent).
export function scoreFlat(values) {
  let total = 0;
  for (let i = 0; i < values.length; i++) {
    total = total + values[i];
  }
  return total;
}
