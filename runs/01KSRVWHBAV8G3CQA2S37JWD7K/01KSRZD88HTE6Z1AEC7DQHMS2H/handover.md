## Done

- Flipped `rubix/docs/proposal/fe-cache-opt-in.md` Status from `Deferred` to `Landed (v1 + v2 + v3 — 2026-Q3)`; rewrote "Why this is deferred" as "Why this landed" naming the three un-defer conditions (workload arrived; `WarehouseWriter` chokepoint shipped in stage 3; agent-step is the non-cache `starter-windowed` consumer); converted "Minimum viable v0" → "v0 baseline (shipped 2026-05-29)" linking to `cache-v0-progress.md`; removed a duplicated `### What changed in this revision` heading.
- Created `rubix/docs/sessions/cache-v1-v2-v3-progress.md` following the v0 doc structure: Status, Scope, Decisions log (13 entries covering SWR caller-driven, empty_ttl semantics, bucket-tag shape, read_only catalog, starter-windowed dep boundary, per-engine fetcher placement, align_to=utc only, inner_scope/EvalContext, SDUI additive IR, EventBus apply_remote, Valkey staged shape, warmer opt-in, dimension-tag opt-in, WarehouseWriter dedup), and one session entry per stage (1, 2, 3, 4).
- Reconciled `rubix/docs/operations/cache-runbook.md` "Anatomy of the response" against the real `SpecRow` (added v1 swr/empty/event/buckets, v2 time_series/inner_scope, v3 invalidator_kind + top-level warmer); replaced the v0 "What this cache does NOT do" list with a post-v3 coverage list pointing at both progress docs; added "Authoring a windowed sidecar", "Authoring two-layer caching", and "Authoring dimension-scoped tags" sections; updated the schema-migration playbook to reflect the v3 chokepoint firing automatically.
- Committed as `dd26010 stage 4 — proposal status flip + final reconciliation`.

## Next

- (none) — this is the documentation capstone; the job is complete.

## What you need to know

- This stage is docs-only — no `.rs` files were touched, so `cargo doc` / `cargo test --doc` are unaffected by my changes.
- `cargo fmt --check` produces ~100 pre-existing diffs at HEAD (`starter-auth-users`, `starter-mcp`, `rubix-agent` supervisor code, etc.) that predate stage 4 — verified by stashing my doc changes and re-running. Not in scope for a docs-only stage to fix; flagging here for visibility.
- The runbook now treats v1/v2/v3 as the operator-facing baseline; the "v3 — …" subsections that were already in place (event-bus, Valkey, warming, dimension tags, WarehouseWriter, SDUI integration) are retained verbatim.
- The proposal still preserves all original design content; only the Status, the deferral retrospective, and the v0 section were rewritten. "What changed in this revision" closing sentence now reads "is the design that landed in v1+v2+v3" instead of "if and when the un-defer conditions fire".

## Open questions

- (none)
