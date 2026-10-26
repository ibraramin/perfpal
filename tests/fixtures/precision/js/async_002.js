// Precision corpus — JS sync call in async (PERF-JS-ASYNC-002): planted TP.
import { readFileSync } from "node:fs";

export async function readConfig(path) {
  return readFileSync(path, "utf8");
}
