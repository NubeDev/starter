# @nube/starter-ui-ir

Hand-written TypeScript mirror of the [`starter-ui-ir`](../../crates/starter-ui-ir/)
Rust crate. The Rust crate owns the schema; this package re-states
the narrow wire shapes (resolve/action requests + responses,
`UiComponent`, `ShowWhen`, subscription/write plans) so TS consumers
need only one dependency to type SDUI payloads.

## Why this exists

Scope OQ-7 (`rubix/docs/scope/dashboards/`) flagged that the renderer
package wants an `@nube/starter-ui-ir` TS surface but only the Rust
crate ships today. This package is the small upstream addition that
unblocks `@nube/starter-ui-sdui-react`. Long term it is generated
from the committed JSON Schema at
`crates/starter-ui-ir/schema/starter-ui-ir.schema.json`; today it is
hand-maintained — keep it boring.

## What it exports

- `IR_VERSION` — single supported IR major version.
- `UiComponent`, `UiComponentTree`, `Kind`, `NodeStyle`, `ShowWhen`.
- `UiResolveResponse` + ok/dry-run variants, `SubscriptionPlan`,
  `SubscriptionSubject`, `WritePlanEntry`.
- `UiActionResponse` (toast / redirect / patch / full_render /
  dialog / dismiss_dialog / diagnostics / open_url / noop).
- `ResolveRequest`, `ActionRequest`, `TableRequest`, `TableResponse`,
  `ClientCapabilities`, `Diagnostic`, `OptimisticHint`,
  `UiTableRow`.

## When you change the Rust crate

1. Regenerate the JSON Schema artifact (`cargo run --bin
   emit_schema -p starter-ui-ir`).
2. Mirror the change into `src/index.ts`. Keep types narrow —
   the renderer treats unknown fields opaquely.
3. Bump `IR_VERSION` only when the Rust side does.
