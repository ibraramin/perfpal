// Entry module declared by package.json `main` (entry-declaration ladder:
// declared target ⇒ script / HIGH). Deliberately free of rule inputs.
import { summarize } from "./13_controls.js";

export function boot(matrix) {
  return summarize(matrix);
}
