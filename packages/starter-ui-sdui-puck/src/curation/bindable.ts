// Binding-eligible fields — typed leaves that the IR's binding
// grammar also accepts as `{{$page.x}}` (or `$user.*`, `$stack.*`).
//
// The schema doesn't carry this signal — it's curated next to the
// generator per scope §B1. v1 covers `$page` only, so the list is
// short. Each entry is a `(variantType, propertyPath)` tuple.
//
// In PR1 this table is *read* by the generator (so it stays a
// single source of truth) but the BindingAwareField wrapper itself
// is not yet built — bindable fields currently render as the
// default typed editor. PR2+ wires the toggle.

export type BindableTuple = {
  variant: string;
  /** Dotted path inside the variant's props. */
  propertyPath: string;
};

export const BINDABLE: readonly BindableTuple[] = [
  // Chart range — both ends drive a `/ui/resolve` refresh when bound.
  { variant: "chart", propertyPath: "range.from" },
  { variant: "chart", propertyPath: "range.to" },
  // Drawer open/closed state, driven by $page key.
  { variant: "drawer", propertyPath: "open" },
  // KPI numeric value can read from a $page slot.
  { variant: "kpi", propertyPath: "value" },
] as const;

/** True when (variant, propertyPath) accepts a binding string. */
export function isBindable(variant: string, propertyPath: string): boolean {
  return BINDABLE.some(
    (b) => b.variant === variant && b.propertyPath === propertyPath,
  );
}
