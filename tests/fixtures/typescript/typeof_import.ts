// A `typeof import("./…")` type position must not fire a finding even if the
// pinned grammar marks the file partial. It is a type query, not a runtime
// dynamic import, so it is also not an IncludeEdge.
export type TargetModule = typeof import("./typeof_target");

export type TargetKeys = keyof TargetModule;
