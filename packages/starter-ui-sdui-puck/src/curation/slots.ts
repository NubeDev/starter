// Layout drop-target slots — the (variantType, propertyName) tuples
// whose `children`-array is a Puck **slot** (drag-target on the
// canvas), not a typed array field.
//
// Concrete v1 list lifted verbatim from
// rubix/docs/scope/dashboards/10-puck-builder.md §B1. Keep this list
// hand-curated and greppable — do not generate it from the schema.
// Any layout container whose children should accept drops MUST be
// listed here; missing it makes children render as a JSON array
// field (visible breakage, not silent).

export type SlotTuple = {
  /** Snake-case `type` discriminator of the parent IR variant. */
  variant: string;
  /**
   * Property path inside the variant whose value is a `Vec<Component>`.
   * Plain `"children"` for the common case; `"tabs[].children"` for
   * the per-tab children slot inside `Tabs.tabs: Vec<Tab>`.
   */
  propertyPath: string;
};

export const SLOTS: readonly SlotTuple[] = [
  { variant: "page", propertyPath: "children" },
  { variant: "row", propertyPath: "children" },
  { variant: "col", propertyPath: "children" },
  { variant: "grid", propertyPath: "children" },
  { variant: "card", propertyPath: "children" },
  { variant: "section", propertyPath: "children" },
  // Nested: `Tabs.tabs: Vec<Tab>` where each Tab has its own
  // `children: Vec<Component>`. PR1's generator recognises this
  // tuple for the assertion in B2 but still renders `tabs` as an
  // authored array of objects (slot-inside-array Puck wiring is
  // tracked for a later PR). The tuple stays here so reviewers see
  // intent and tabs.tabs[].children is never treated as an authored
  // Component array elsewhere.
  { variant: "tabs", propertyPath: "tabs[].children" },
] as const;

/** True when (variant, propertyName) is a layout slot. */
export function isSlot(variant: string, propertyName: string): boolean {
  return SLOTS.some(
    (s) => s.variant === variant && s.propertyPath === propertyName,
  );
}
