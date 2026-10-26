// TSX generic-arrow disambiguation: both the portable `<T,>` form and the bare
// `<T>` form parse cleanly in expression position on the pinned grammar. This
// module must stay silent (no loop, no async, no accumulation).
export const identity = <T,>(value: T): T => value;

export const identityBare = <T>(value: T): T => value;
