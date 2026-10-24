// Precision corpus — JS non-async silent control (must stay silent).
import { readFileSync } from "node:fs";

export function readConfigSync(path) {
  return readFileSync(path, "utf8");
}
