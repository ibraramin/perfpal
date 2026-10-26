// Entry module declared by package.json `main` (entry-declaration ladder:
// declared target ⇒ script / HIGH). Deliberately free of rule inputs.
import { summarize } from "./13_controls";
import type { Matrix } from "./04_deep_nesting";

export function boot(matrix: Matrix): number {
  return summarize(matrix);
}
