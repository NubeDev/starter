## Done

- Reviewed cumulative cache v1+v2+v3 diff (62961ab..b2206c2: stages 1, 2, 4) against Layer-1 invariants
- Confirmed R1 — `starter-cache` deps: starter-windowed + workspace utils only; `starter-windowed` has zero `starter-cache` edge; `rubix-agent` is the consumer that supplies the `InvalidationBus` adapter
- Confirmed R2 — `EventBusInvalidator` uses an abstract trait; the host adapter reuses the existing `RubixEventBus`; Valkey backend ships as a mock behind a feature flag; tower layer wraps the existing axum app
- Confirmed R4/R5 — `/ui/action` never cached, action dispatch still goes through the tool registry; cache keys carry `CallerScope { tenant, user }` + `units_hash`; two-layer `inner_scope:` only relaxes the units-conversion seam, not authz
- Confirmed wire-formats — `ComponentTree.cache` is additive `Option` with `skip_serializing_if`, IR_VERSION unchanged; admin SpecRow grows only additive optional fields; `WarehouseWriteBackend::insert` is an internal trait, not wire
- Verified the five v0 `// TODO(cache-invalidation):` markers are gone from source files; the `WarehouseWriter::commit` chokepoint is wired at `rubix/crates/rubix-agent/src/extensions/warehouse_write.rs` and enforced by the type system
- Wrote `rubix/docs/sessions/cache-v3-review.md` and committed as `stage 6 — v3 Layer-1 review: PASS …` (commit 8b994d6)

## Next

- (none — Layer-1 gate passes; a later ramp step can address the flagged follow-ups below)

## What you need to know

- Verdict sentinel for the runtime: `PASS: R1/R2/R4/R5 + wire-format invariants hold across v3 cumulative diff`
- Three functional follow-ups flagged in the review doc but NOT Layer-1 fails: (1) Valkey backend is a shape-correct mock pending a one-file swap to real `redis://`; (2) cold-start warmer no-ops on the very first boot because top-N stats are zero — worth a runbook line; (3) dimension-scoped tag cardinality is bounded only by operator config — a `device_uuid` tag would explode the tag table, worth a cardinality warning in the runbook's dimension-scoped-tags section
- Stage commit chain on the branch: 62961ab (v1) → 7f63319 (v2) → eb6f021 (v3) → 8b994d6 (this review). The review only adds docs; no code touched

## Open questions

- (none — flagged items are documentation follow-ups, not blockers)
