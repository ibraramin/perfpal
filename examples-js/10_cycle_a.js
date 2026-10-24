// PERF-JS-ARCH-001 — import cycle, edge A of the pair. `10` and `11` import
// each other; the resolver keeps the lowest-sorted edge (10 -> 11) and breaks
// 11 -> 10, so the finding anchors in `11_cycle_b.js`.
import { formatFromB } from "./11_cycle_b.js";

export function format(value) {
  return formatFromB(value);
}
