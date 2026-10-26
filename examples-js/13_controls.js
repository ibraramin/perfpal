// Cross-cutting silent controls. The import is ACYCLIC (this module imports
// `04_deep_nesting.js`, which imports nothing), so PERF-JS-ARCH-001 stays
// silent here; the `.concat`/`.slice` calls sit outside loops or never
// reassign their receiver.
import { score } from "./04_deep_nesting.js";

export function chunk(items, size) {
  const chunks = [];
  for (let i = 0; i < items.length; i = i + size) {
    chunks.push(items.slice(i, i + size)); // silent: receiver is not reassigned
  }
  return chunks;
}

export function summarize(items) {
  return score(items); // silent: acyclic import edge
}

export function mergeDefaults(base, patch) {
  return base.concat(patch); // silent: outside every loop
}
