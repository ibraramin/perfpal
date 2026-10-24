// PERF-JS-ARCH-001 — import cycle, edge B of the pair (this edge is the one
// break_cycles drops, so the Info finding is anchored on line 3).
import { format } from "./10_cycle_a.js";

export function formatFromB(value) {
  return format(value);
}
