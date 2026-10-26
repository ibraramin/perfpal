// Type-land-only module: interfaces, type aliases and `declare` ambient
// declarations. Nothing here is runtime code, so the scanner must emit no
// calls, allocations or loops, and no function record beyond `<module>`.
export interface Point {
  x: number;
  y: number;
}

export type Pair<T> = readonly [T, T];

export declare function ambient(value: number): number;

export declare class Ambient {
  run(): void;
}
