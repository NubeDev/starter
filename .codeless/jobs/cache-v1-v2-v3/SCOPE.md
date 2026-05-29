# Scope — cache-v1-v2-v3

The authoritative design lives at
[/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md](/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md).
The v0 baseline this job builds on is documented at
[/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md](/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md)
and the operator surface at
[/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md](/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md).

Where this brief disagrees with the proposal, **the proposal wins** —
fix this file rather than diverge.

## Goal

Land every deferred feature of the opt-in caching proposal on top of
the already-shipped v0. After this job:

1. **v1** — SWR (`stale_while_revalidate` + `max_stale`),
   `cache_empty` / `empty_ttl`, bucket-level invalidation tags
   (`bucket:<table>:<floor(t, granularity)>`), write-path event tags
   (`event:<name>`), and the read-only handler declaration mechanism
   are all live at the existing kind dispatcher seam. The v0
   sidecar shape grows the new optional keys; existing v0 sidecars
   keep parsing.

2. **v2** — A new `crates/starter-windowed` companion crate ships
   (engine-agnostic — zero dep on `starter-cache`). The cache spec
   grows an optional `time_series:` block plus `inner_scope:` for
   two-layer (tenant-shared + user-rendered) caching. Per-engine
   fetchers ship: `TimescaleWindowedFetcher` in
   `starter-store-warehouse`, `PgWindowedFetcher` in
   `starter-store-postgres`. The worked example D from the proposal
   (7d / 90d delta-fetch on `usage_bucketed` with two-layer
   coalescing) is end-to-end green against the dev rig.

3. **v3** — Distribution and scale. SDUI integration (page-level
   `cache:` block on the IR root, additive — no IR version bump,
   `/ui/resolve` + `/ui/table` caching live, `/ui/action` never
   cached but action handlers fire invalidate via the v1 read-only
   mechanism). Core HTTP tower layer (`CacheLayer::tower()`).
   Event-bus invalidator behind the `Invalidator` trait, picked by
   `RUBIX_CACHE_INVALIDATOR=local|event-bus`. Valkey backend behind
   a `valkey` cargo feature. Cold-start warmer driven by the
   per-spec stats top-N. Dimension-scoped tags
   (`table:<name>:<dim>=<value>`). A unified `WarehouseWriter`
   chokepoint replaces the five scattered
   `// TODO(cache-invalidation):` markers from v0 — invalidation
   becomes type-system-enforced rather than lint-and-hope.

4. The proposal at `rubix/docs/proposal/fe-cache-opt-in.md` flips
   `Status: Deferred` → `Status: Landed (v1 + v2 + v3 — 2026-Q3)`;
   a new `rubix/docs/sessions/cache-v1-v2-v3-progress.md` is the
   per-stage progress log with the same shape as the v0 doc; the
   operator runbook covers every new surface.

## In scope (three implementation phases + a documentation capstone)

- **Stage 1 — v1 (existing dispatcher seam)**: SWR + `cache_empty` /
  `empty_ttl` + bucket-tag registry + event-tag registry + read-only
  handler declaration + dispatcher auto-invalidate on writing-handler
  success. New parser keys land in the existing hand-rolled YAML
  parser with the v0 line-number error path. Canary sidecar grows an
  opt-in `stale_while_revalidate: 30s`.
- **REVIEW gate** between v1 and v2.
- **Stage 2 — v2 (`starter-windowed` + `time_series:` +
  `inner_scope:`)**: new engine-agnostic crate, per-engine fetchers,
  cache integration via thin adapter, two-layer caching using the
  existing `starter-i18n` / `starter-spi::preferences` units stack.
  Worked example D wired to the canary end-to-end.
- **REVIEW gate** between v2 and v3.
- **Stage 3 — v3 (distribution + SDUI + tower + chokepoint)**: SDUI
  resolve / table caching, core HTTP tower layer, event-bus
  invalidator, Valkey backend, cold-start warmer, dimension-scoped
  tags, **the `WarehouseWriter` chokepoint** that retires the five
  scattered TODO markers.
- **REVIEW gate** after v3.
- **Stage 4 — documentation capstone**: flip the proposal status,
  write the v1+v2+v3 progress doc, expand the operator runbook.

## Out of scope

- **Backwards compatibility with v0 wire shape beyond additive
  growth.** Every new key is optional with a default that matches
  v0 semantics — an existing v0 sidecar must keep parsing. We do
  not, however, support reading a v1 sidecar with a v0-shaped
  enum; this is a non-issue because every spec is parsed locally.
- **A second `Cache` backend beyond Valkey.** `foyer` is mentioned
  in the proposal as the "larger-than-RAM" future; we do not ship
  it here.
- **A second `DdlDialect`-style engine swap on `WindowedFetcher`.**
  Only Timescale and Postgres impls ship; the trait shape is
  designed to allow others but none beyond those ship here.
- **A general-purpose event-bus replacement.** The event-bus
  invalidator rides on top of the existing `RubixEventBus`; we do
  not introduce a new bus or refactor the existing one.
- **Backfilling cache entries from cold-disk state.** The cold-start
  warmer replays *cache keys* (by spec-id top-N), not warehouse
  reads against arbitrary historical windows.
- **Promoting hot tag keys to dimension columns.** The v3 dimension
  tag is a string concatenation (`<dim>=<value>`) registered at
  writer setup; we do not change the underlying column shape.
- **Anything past v3 — D-NP territory.** No
  cross-process-replica-coherence-beyond-tag-fan-out, no
  read-your-write distributed semantics, no offline-first SDUI.
- **MCP-only refactor of the AI surface.** Out of scope for this
  job; tracked separately.
- **The starter-extensions sibling workspace's own refactors.** We
  add deps and surface but do not refactor unrelated bits.

## Constraints

- **v0 must keep working at every stage boundary.** Every existing
  v0 test stays green (starter-cache: 21 unit + 7 scenario + 1
  canary; starter-ext-server: 18 unit + 5 cache_wrap; rubix-agent:
  12 admin_cache_test). New tests are additive.
- **R1** — 400 lines per file. Apply to every new module created
  in `starter-cache`, `starter-windowed`, `starter-sdui-routes`
  cache-integration code, etc.
- **R4** — layer arrow `contracts → domain → transport`. The
  `WindowedFetcher` trait lives in `starter-windowed` (contracts);
  the Timescale / PG impls live in the store crates (domain); the
  cache-integration adapter lives where the call site is (transport).
- **R10** — add-only-within-a-major. Every new `CacheSpec` field is
  optional and back-compatible; the YAML parser does not break v0
  sidecars; the IR `cache:` block is purely additive (no IR
  version bump per §"What changed in this revision").
- **R11** — tests live with the code. Every new spec field, every
  new endpoint, every new fetcher impl, the chokepoint, the
  warmer, and the event-bus invalidator gets unit + integration
  coverage. The existing `MockClock` + `TracingCache<C>` test
  infrastructure extends to v1's SWR semantics.
- **R12** — comments explain why, never what. No `// renamed from
  v0` graveyard markers, no `// TODO(v2):` after v2 ships.
- **R13** — drive everything through mani. `mani run build`,
  `mani run test`, `mani run lint`, `mani run status` are the
  contributor entry points across all four stages.
- **starter-cache stays `#![forbid(unsafe_code)]`.** The v0
  invariant carries through; if a v1+ feature requires unsafe, the
  feature is wrong-shaped.
- **The hand-rolled YAML parser stays.** No pulling `serde_yaml`
  into the workspace dep surface — the v0 decision to keep it
  hand-rolled was deliberate. The parser grows new keys with the
  v0 line-number error machinery.
- **starter-extensions sibling-workspace plumbing.** Adding a dep
  to `starter-ext-server` means updating both
  `starter-extensions/Cargo.toml`'s `[workspace.dependencies]`
  block **and** the per-crate Cargo.toml — verified in v0 when
  `starter-cache` was first added there. The same shape applies
  for any new path-dep this job introduces across the workspace
  boundary.
- **The v0 dispatcher hard rules survive intact.** Streaming
  dispatch never wraps the cache (the
  `streaming_dispatch_bypasses_cache_even_with_sidecar` test is
  the regression fence). `dispatch_base_key`'s object-key
  canonicalisation invariant (`base_key_canonicalises_object_key_order`)
  is load-bearing for hit rate — if serde_json's `preserve_order`
  feature ever needs to land, that test trips first.
- **ADR-003 tenancy.** Every new cache layer respects per-tenant
  partitioning. `inner_scope: tenant` does not bypass tenancy —
  the inner cache is still per-tenant; only the *user-scoped key*
  collapses to tenant-shared.
- **No `--force`, no `--no-verify`.** If a hook fails, fix the
  cause.

## Open questions — to resolve in writing at REVIEW gates

These are explicit so they get answered on disk, not silently
guessed. Stage 1 may surface answers to some of them as a
side-effect of implementing v1; bring those answers into the
REVIEW gate handover.

### Q1 — SWR: does an in-flight refresh extend or replace the stale window?

The proposal §Layer 6b is explicit: serve stale, kick off exactly
one background refresh, all concurrent callers receive stale until
refresh completes. Verify the implementation matches in stage 1;
surface at the REVIEW gate if the test scenarios reveal a different
shape (e.g. "refresh-during-refresh" — what happens on a second
concurrent miss while the first is still loading?).

### Q2 — Bucket granularity collision

A spec on table `X` declaring `bucket: 1h` while another spec on
table `X` declares `bucket: 15m` means the writer must fan-out to
*every* declared granularity on every row. The registry knows which
granularities exist. Verify at stage 1 that the write-path code
emits the **set** of `bucket:<table>:<floor(t, g)>` tags for every
registered `g`, not just the first one. Surface at the REVIEW gate
if any spec in the codebase reveals a different shape.

### Q3 — `starter-windowed` reusability without `starter-cache`

The proposal says `starter-windowed` should be its own proposal
"shipped independently of the cache layer, with that consumer
driving the trait shape". For this job we ship it inside the cache
work — but the trait shape must remain reusable. Verify at stage 2
that *at least one* non-cache consumer exists in the workspace
(candidates: a flow node that does delta-fetch, an agent step that
exports time-windowed data). If no such consumer exists, raise it
at the REVIEW gate — landing `starter-windowed` without a
non-cache consumer is exactly the "premature platform" failure
mode the proposal calls out.

### Q4 — Two-layer caching: where does convert-on-read live?

`inner_scope: tenant` caches the canonical-units result; the outer
layer re-renders per user prefs. The conversion needs `EvalContext`
or equivalent — verify at stage 2 that the dispatcher hands the
layer enough context to run the conversion. Surface at the REVIEW
gate if the EvalContext flow doesn't naturally extend into the
dispatcher seam.

### Q5 — SDUI cache key components for `/ui/resolve`

The proposal §Layer 2 lists `(tenant, user, page_id, target_ref,
stack_hash, page_state_hash, units_hash)` as the resolve-cache key,
plus per §"Failure modes" `ir_version` + per-page content hash for
schema-change invalidation. Verify at stage 3 that every component
is in the key and that a binding-grammar change (`{{$target/...}}`)
forces a re-key naturally. Surface if any per-target dimension is
not falling out of `target_ref` automatically.

### Q6 — Event-bus delivery semantics

`RubixEventBus` is `tokio::sync::broadcast` today; a slow consumer
can lag and miss events. For invalidation, missed events =
stale-forever entries. Verify at stage 3 whether the existing bus
satisfies the at-least-once requirement, or whether the v3
invalidator needs to layer a persistence shim. Surface at the
REVIEW gate; the resolution may be "switch to a persistent bus" or
"accept TTL as the fallback for lagged consumers".

### Q7 — `WarehouseWriter` chokepoint surface

The proposal §Layer 3 calls for a chokepoint that every write goes
through. The v0 TODO markers are at five sites — four in
`starter-store-warehouse/src/tsdb/store/*.rs`, one in
`rubix/crates/rubix-agent/src/extensions/warehouse_write.rs`. The
chokepoint shape (one trait? one struct? per-table writers behind
one façade?) is a contract decision; surface the chosen shape at
the v3 REVIEW gate, with a paragraph in the progress doc justifying
why this shape and not the alternatives.

### Q8 — Valkey backend ownership

Valkey is BSD-3 (Redis fork) — verify license compatibility with
the workspace. Verify the connection-pooling story (the existing
sqlx pool is per-host; Valkey wants its own connection manager).
Surface at the v3 REVIEW gate if either is non-obvious.

### Q9 — Cold-start warmer triggers

The warmer replays "top-N cache keys after deploy" — but the
per-spec stats are *spec-id* counters, not key counters. Storing
key-level top-N for warming means writing the per-key access
counts to disk (cheap, but new state). Surface the chosen approach
at the v3 REVIEW gate — spec-id-driven replay vs key-driven
replay; the proposal stays at the spec-id level so that is the
default unless stage 3 surfaces a reason to drill down.

## Deliverables (what "done" looks like)

1. `codeless/cache-v1-v2-v3` branch with one commit per stage
   (four stages = four commits), pushed via mani. Three REVIEW
   gates sit between the stages and produce handover updates but
   no commits of their own beyond the trio.
2. `cargo build --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-features -- -D warnings` green
   at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. `mani run build --all` green; `mani run lint` green; `mani run
   test --all` green at every stage boundary.
6. v0 tests stay green at every stage boundary (regression fence).
7. New tests added per stage as listed in the per-stage description
   in `template.yaml` — verify each named scenario is present.
8. Five scattered `// TODO(cache-invalidation):` markers from v0
   are gone after stage 3 (replaced by the `WarehouseWriter`
   chokepoint firing `invalidate_tags` automatically).
9. `rubix/docs/proposal/fe-cache-opt-in.md` Status flipped from
   `Deferred` to `Landed (v1 + v2 + v3 — 2026-Q3)` in stage 4.
10. `rubix/docs/sessions/cache-v1-v2-v3-progress.md` exists with
    one session entry per stage and a decisions log capturing every
    non-obvious choice, mirroring the structure of the v0 progress
    doc.
11. `rubix/docs/operations/cache-runbook.md` covers the full v3
    surface — multi-node invalidation, Valkey, warmer, SWR,
    `time_series:`, `inner_scope:`, dimension-scoped tags, the
    `WarehouseWriter` chokepoint behaviour from an operator's POV.
12. `GET /api/v1/admin/cache/specs` SpecRow covers every new field
    additively; runbook's "Anatomy of the response" section is
    accurate against the actual shape.
13. The canary sidecar at
    `rubix/extensions/com.nubeio.rubixos/kinds/com.nubeio.rubixos.warehouse_query.cache.yaml`
    grows opt-in lines per stage (v1 `stale_while_revalidate`, v2
    `time_series:` + `inner_scope:`, v3 nothing — v3 is platform-
    level not author-level for the canary). The canary smoke test
    in `crates/starter-cache/tests/canary_sidecar.rs` updates with
    each shape change.

## References

- Proposal (authoritative):
  [/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md](/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md).
- v0 progress (what we're building on):
  [/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md](/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md).
- v0 operator runbook (the surface this job extends):
  [/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md](/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md).
- Parent rubix SCOPE for R0–R13:
  [/home/user/code/rust/starter/rubix/SCOPE.md](/home/user/code/rust/starter/rubix/SCOPE.md).
- starter-extensions sibling workspace (where the dispatcher
  integration lives):
  [/home/user/code/rust/starter/starter-extensions/](/home/user/code/rust/starter/starter-extensions/).
- mani docs in the codeless-workspace:
  [/home/user/code/rust/codeless-workspace/DOCS/MANI.md](/home/user/code/rust/codeless-workspace/DOCS/MANI.md).
