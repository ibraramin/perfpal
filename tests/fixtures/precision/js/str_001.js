// Precision corpus — JS string build (PERF-JS-STR-001): planted TP.
export function render(parts) {
  let out = "";
  for (const part of parts) {
    out += `part:${part}`;
  }
  return out;
}
