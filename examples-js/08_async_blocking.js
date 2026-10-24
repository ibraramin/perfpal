// PERF-JS-ASYNC-002 — synchronous blocking call inside an `async` function
// (Warn). True positive: `readFileSync` in an async function. Silent control:
// the same sync API in a NON-async function (the async gate is mandatory).
import { readFileSync } from "node:fs";

export async function readConfig(path) {
  return readFileSync(path, "utf8"); // TP PERF-JS-ASYNC-002
}

export function readConfigSync(path) {
  return readFileSync(path, "utf8"); // silent: non-async function
}
