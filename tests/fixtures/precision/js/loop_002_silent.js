// Precision corpus — JS sync I/O silent control (must stay silent).
import { readFileSync } from "node:fs";

export function loadOnce(file) {
  return readFileSync(file, "utf8");
}
