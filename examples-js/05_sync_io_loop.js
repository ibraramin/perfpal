// PERF-JS-LOOP-002 — synchronous blocking I/O inside a loop (Warn).
// True positive: `readFileSync` per iteration. Silent control: the same sync
// API outside every loop (not per-iteration churn).
import { readFileSync } from "node:fs";

export function loadAll(files) {
  const chunks = [];
  for (const file of files) {
    chunks.push(readFileSync(file, "utf8")); // TP PERF-JS-LOOP-002
  }
  return chunks;
}

export function loadOnce(file) {
  return readFileSync(file, "utf8"); // silent: outside every loop
}
