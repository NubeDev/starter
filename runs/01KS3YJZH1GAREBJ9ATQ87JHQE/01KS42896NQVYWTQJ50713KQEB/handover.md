## Done

- reviewed stages 2–7 diff against SCOPE R1/R2/R4/R5 and DIVERGENCE.md
- confirmed crate dep direction, capability/version handshake, server-side binding eval, single /ui/action endpoint with diagnostics rename
- emitted PASS sentinel

## Next

- Stage 9: extended-component phase per WORKFLOW (build on frozen core, no signature changes)

## What you need to know

- starter-server still does not depend on starter-sdui-routes (D4 invariant intact)
- form_errors is rejected at the wire by an active unit test in crates/starter-ui-ir/src/action.rs — do not weaken it
- core surface is frozen from this stage forward; any future Phase 6/7 need that would touch Component IR, EntityGraph, builder DSL, Renderer dispatcher, HandlerRegistry/QueryEngine, or the three /ui/* routes must stop and propose a core change explicitly

## Open questions

- (none)

PASS: Layer-1 invariants hold — ui-ir is I/O-free, bindings/builder depend only on ir, starter-server does not import starter-sdui-routes, /ui/action is the sole interaction endpoint with the diagnostics-renamed discriminated union, and the capability/ir_version handshake is wired on both ends.
