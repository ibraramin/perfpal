// Precision corpus — JS nested loops (PERF-JS-LOOP-001): planted TP.
export function score(matrix) {
  let total = 0;
  for (let row = 0; row < matrix.length; row++) {
    for (let col = 0; col < matrix[row].length; col++) {
      total = total + matrix[row][col];
    }
  }
  return total;
}
