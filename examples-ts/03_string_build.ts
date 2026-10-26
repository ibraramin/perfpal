// PERF-JS-STR-001 — string accumulation in a loop (Warn).
// TP: unknown trip, string/template RHS. Silent: tiny loop (< 8), out-of-loop,
// and numeric `+=` (the B2-W0 string gate).

export function render(parts: readonly string[]): string {
  let out = "";
  for (const part of parts) {
    out += `part:${part}`; // TP PERF-JS-STR-001
  }
  return out;
}

export function tinyPrefix(items: readonly string[]): string {
  let out = "";
  for (let i = 0; i < 5; i++) {
    out += `item:${items[i]}`; // silent: canonical trip count 5 < 8 (string RHS)
  }
  return out;
}

export function appendOnce(s: string, suffix: string): string {
  s += suffix; // silent: outside every loop
  return s;
}

export function sumValues(values: readonly number[]): number {
  let total = 0;
  for (const value of values) {
    total += value; // silent: numeric += is not a string build
  }
  return total;
}
