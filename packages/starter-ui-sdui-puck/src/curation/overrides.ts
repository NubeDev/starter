// Hand-written ComponentConfig overrides for IR variants whose shape
// the auto-generator cannot handle cleanly. v1 list, per
// rubix/docs/scope/dashboards/10-puck-builder.md §B1:
//
//   * `repeat`  — `template: Box<Component>` (nested polymorphic)
//   * `wizard`  — `steps: Vec<…>` with per-step children
//   * `form`    — bound `fields` array + submit `Action`
//   * `table`   — toolbar / row-actions referencing `Action`
//
// PR1 keeps every entry `null` — placeholder. PR2/PR3 fill these in
// once the real renderer / selectors land. Until then the generator
// emits each of these variants with the default mapping; the override
// hook is present so later work has a clean drop-in point.
//
// HARD RULE (asserted at module load): the resolver-only variants —
// `forbidden`, `dangling`, `unknown` — MUST NEVER appear as keys in
// this map. They are emitted by the server's resolve path and are
// not author-time tiles (scope §B2). The assertion below makes the
// rule mechanical so a future contributor can't accidentally
// re-introduce them.

import type { PuckComponentConfigStub } from "../puck-types.js";

/**
 * Override map: snake-case IR `type` → Puck `ComponentConfig`. A
 * `null` entry means "no override yet — fall through to the
 * auto-generator." Listed-but-null entries document the intent that
 * a real override is coming later.
 */
export const OVERRIDES: Readonly<Record<string, PuckComponentConfigStub | null>> = {
  // TODO(PR2+): hand-author the per-item template editor for Repeat.
  repeat: null,
  // TODO(PR2+): hand-author the steps array + per-step children
  // slots for Wizard.
  wizard: null,
  // TODO(PR2+): hand-author the bound-fields + submit-Action surface
  // for Form once the action picker (B3) lands.
  form: null,
  // TODO(PR2+): hand-author toolbar / row-action surfaces for Table
  // once the action picker (B3) lands.
  table: null,
};

/**
 * Resolver-only variants — these MUST NOT appear as authoring tiles
 * or override keys. Scope §B2.
 */
export const RESOLVER_ONLY_VARIANTS: readonly string[] = [
  "forbidden",
  "dangling",
  "unknown",
] as const;

// Mechanical guard: assert at module load that no resolver-only
// variant is registered in OVERRIDES. This runs once per import and
// throws loudly rather than letting CI catch it later. Listing one
// of these (even as `null`) is wrong because the palette generator
// treats presence-in-map as "this is an authored variant."
for (const banned of RESOLVER_ONLY_VARIANTS) {
  if (Object.prototype.hasOwnProperty.call(OVERRIDES, banned)) {
    throw new Error(
      `[starter-ui-sdui-puck/curation/overrides] resolver-only variant "${banned}" must not appear in OVERRIDES — see scope §B2.`,
    );
  }
}
