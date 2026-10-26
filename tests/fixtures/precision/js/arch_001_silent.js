// Acyclic control: imports a module that does not import this one back.
import { helperB } from "./arch_001_b.js";

export function wrap(value) {
  return helperB(value);
}
