// A typed JSX component: JSX is parsed structurally and only embedded
// expressions contribute events. No loop, no async, no accumulation — this
// module must stay silent (plan `js-ts` §12).
export interface BadgeProps {
  label: string;
  count: number;
}

export function Badge({ label, count }: BadgeProps) {
  return <span title={label}>{count > 0 ? label : "none"}</span>;
}
