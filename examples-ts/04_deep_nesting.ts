// PERF-JS-LOOP-001 — nested loop depth >= 2 (Info). True positive: a
// two-deep nest. Silent control: a single-level loop that stays at depth 1.
// `Matrix` is re-imported type-only by `14_types_only.ts` (erasure control).

export type Matrix = number[][];

export function score(matrix: Matrix): number {
  let total = 0;
  for (let row = 0; row < matrix.length; row++) {
    for (let col = 0; col < matrix[row].length; col++) {
      total = total + matrix[row][col]; // TP PERF-JS-LOOP-001
    }
  }
  return total;
}

export function scoreFlat(values: readonly number[]): number {
  let total = 0;
  for (let i = 0; i < values.length; i++) {
    total = total + values[i]; // silent: loop depth stays at 1
  }
  return total;
}
