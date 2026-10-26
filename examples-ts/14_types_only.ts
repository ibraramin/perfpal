// TypeScript-only constructs that are ERASED at runtime. This module must
// produce zero findings: interfaces/type aliases/`declare`/overloads emit no
// records or events, the `import type` edge stays out of the include graph
// (ARCH-001 cannot see a type-only cycle), and the module-level `as`/`satisfies`/
// `!` wrapper expressions are not rule inputs (no loop, no async). The dedicated
// event-level erasure lock is `tests/fixtures/typescript/type_only.ts`.
import type { Matrix } from "./04_deep_nesting";

export type Grid = Matrix;
export type Handler = (value: string) => void;

export interface Config {
  retries: number;
  label?: string;
  onRetry: Handler;
}

export declare function ambientParse(text: string): number;

export declare class AmbientCache {
  get(key: string): string | undefined;
}

export function parse(value: string): number;
export function parse(value: number): number;
export function parse(value: string | number): number {
  return Number(value);
}

export enum Mode {
  Fast = "fast",
  Slow = "slow",
}

export namespace Geometry {
  export const unit = 1;
}

export abstract class Shape {
  abstract area(): number;
}

export const config = { retries: 3, label: "default" } as const satisfies Config;
export const raw = JSON.stringify(config) as string;
export const handler = ((value: string) => value.length) satisfies Handler;
export const maybe: string | undefined = raw;
export const definite: string = maybe!;
