// Precision corpus — JS assignment accumulation (PERF-JS-STR-001): planted TP.
export function append(parts) {
  let out = "";
  for (const part of parts) {
    out = out + "!";
  }
  return out;
}
