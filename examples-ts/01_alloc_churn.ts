// PERF-JS-ALLOC-001 — loop-invariant object/array literal inside a loop
// (Info). One true positive plus the two silent controls from the frozen
// disposition table: an induction-variable literal and a literal whose
// referenced name is reassigned inside the same loop.

export interface Envelope {
  status: string;
  retries: number;
}

export function buildEnvelopes(count: number): Envelope[] {
  const envelopes: Envelope[] = [];
  for (let i = 0; i < count; i++) {
    envelopes.push({ status: "new", retries: 0 }); // TP PERF-JS-ALLOC-001
  }
  return envelopes;
}

export function buildIndexed(count: number): { index: number }[] {
  const points: { index: number }[] = [];
  for (let i = 0; i < count; i++) {
    points.push({ index: i }); // silent: depends on the induction variable
  }
  return points;
}

export function buildLabelled(rows: { label: string }[]): { label: string }[] {
  const out: { label: string }[] = [];
  let label = "";
  for (const row of rows) {
    label = row.label;
    out.push({ label }); // silent: `label` is reassigned inside the loop
  }
  return out;
}
