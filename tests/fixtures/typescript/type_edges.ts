// `import type` / `export type` are erased before the module graph: neither
// may produce an IncludeEdge, even when the target is a real scanned file.
import type { Point } from "./type_dep";

export type { Point as PointAlias } from "./type_dep";

export type Coordinate = readonly [number, number];

export function origin(): Point {
  return { x: 0, y: 0 };
}
