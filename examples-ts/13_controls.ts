// Cross-cutting silent controls. The import is ACYCLIC (this module imports
// `04_deep_nesting.ts`, which imports nothing), so PERF-JS-ARCH-001 stays
// silent here; the `.concat`/`.slice` calls sit outside loops or never
// reassign their receiver.
import { score, type Matrix } from "./04_deep_nesting";

export function chunk<T>(items: readonly T[], size: number): T[][] {
  const chunks: T[][] = [];
  for (let i = 0; i < items.length; i = i + size) {
    chunks.push(items.slice(i, i + size)); // silent: receiver is not reassigned
  }
  return chunks;
}

export function summarize(items: Matrix): number {
  return score(items); // silent: acyclic import edge
}

export function mergeDefaults<T>(base: readonly T[], patch: readonly T[]): T[] {
  return base.concat(patch); // silent: outside every loop
}
