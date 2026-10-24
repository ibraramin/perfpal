// Precision corpus — JS sync I/O in a loop (PERF-JS-LOOP-002): planted TP.
import { readFileSync } from "node:fs";

export function loadAll(files) {
  const chunks = [];
  for (const file of files) {
    chunks.push(readFileSync(file, "utf8"));
  }
  return chunks;
}
