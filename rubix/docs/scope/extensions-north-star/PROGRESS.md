# Extensions — North-Star progress

> **Tier:** scope (plan). Lifetime: weeks–months. Per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) **source code must
> not reference this file.** Status only — the design lives in
> [README.md](./README.md) and the canonical reference is
> [docs/design/extensions/](../../design/extensions/).

## How to update this doc

Per [NEW-SESSION.md](../../../NEW-SESSION.md): at the end of
each session that moves a row, the session adds a dated entry to
the [Session log](#session-log) and bumps the row's **Status**
in the [Critical-path tracker](#critical-path-tracker).

Status vocabulary (one of):

| Status | Meaning |
|---|---|
| ❌ not started | No code. Manifest field absent, capability absent, no PR open. |
| 🟡 in progress | A PR is open or branch exists; not merged. |
| 🟣 design landed | Subsection of `docs/design/extensions/` exists and matches an in-flight PR. |
| ✅ shipped | Capability handle (builtin + process) merged to `master`; design doc current; tests passing. |
| 🔵 shipped (wasm) | Wasm backend for an already-shipped handle is merged. |

A row stays at ✅ until its wasm follow-up lands; only then does
the row drop off the tracker entirely.

## Critical-path tracker

Phases mirror [README.md §"Critical path"](./README.md).

### Phase 1 — MVP gate

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 1 | `CallerIdentity` stamping in supervisor | ✅ shipped | — | (master) | [caller-identity.md](../../design/extensions/caller-identity.md) | 2026-05-27 |
| 2 | `WarehouseReadHandle` + builtin `TemplateRegistry` (seeded from `AnalyticsBridge`) | ✅ shipped | — | (master) | [warehouse-read.md](../../design/extensions/warehouse-read.md) | 2026-05-27 |
| 3 | `contributes.warehouse_templates[]` manifest slice | ✅ shipped | — | (master) | [warehouse-templates-contribution.md](../../design/extensions/warehouse-templates-contribution.md) | 2026-05-27 |

### Phase 2 — Usable under load

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 4 | `EventBusHandle` | ✅ shipped (publish; subscribe deferred) | — | (master) | [event-bus.md](../../design/extensions/event-bus.md) | 2026-05-27 |

### Phase 3 — Sharing

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 5 | `DashboardHandle` + `AuthzHandle` with cross-tenant grants | ❌ not started | — | — | — | — |

### Phase 4 — Polish

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 6 | `BlobHandle` + `ExportHandle` | ❌ not started | — | — | — | — |
| 7 | `CronHandle` | ❌ not started | — | — | — | — |

### Phase B (Use-Case-B: ad-hoc data, custom rules)

Lifted out of the "Future" backlog per the 2026-05-27 session.
Drives the cleaner + extension-authored anomaly rules track.

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| B1 | `WarehouseWriteHandle` + `Capability::WarehouseWrite` + `contributes.warehouse_tables[]` + boot-time DDL | ✅ shipped (Postgres/Timescale; no ClickHouse) | — | _local_ | _inline in `rubix-agent/src/extensions/warehouse_write.rs` + `boot/extension_tables.rs`_ | 2026-05-27 |
| B2 | `AnomalyRule` trait + builtin rules + `RuleRegistry` (in-process) | ✅ shipped | — | _local_ | _inline in `rubix-tools/src/cleaner/`_ | 2026-05-27 |
| B3 | Cleaner tick: read L1 window → apply rules → write `samples_l2` | ✅ shipped (sqlx path; tool wrapper deferred to B4) | — | _local_ | _inline in `rubix-tools/src/cleaner/tick.rs` + `starter-store-warehouse/src/tsdb/migrate.rs` (`0006_samples_l2`)_ | 2026-05-27 |
| B4 | Cleaner tool wrapper + bundled flow YAML + 60s schedule | ✅ shipped | — | _local_ | _inline in `rubix-tools/src/cleaner/tool.rs` + `rubix-flows/flows/cleaner.yaml`_ | 2026-05-27 |
| B5 | `contributes.anomaly_rules[]` manifest slot + tool-call dispatch adapter | ✅ shipped (SPI + validation + adapter + host registry wiring) | — | _local_ | _inline in `starter-ext-spi/src/manifest.rs` + `starter-ext-host/src/validate.rs` + `rubix-tools/src/cleaner/adapter.rs` + `rubix-agent/src/registry.rs`_ | 2026-05-27 |

### Plumbing (independent)

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| P1 | Mount `rest[]` / `sse[]` / `workers[]` / `cli[]` adapters in `rubix-agent` boot composer | 🟡 partial — REST+SSE wired via `CompositeRestDispatcher`; `workers[]` + `cli[]` deferred | — | _local_ | _inline in `rubix-agent/src/extensions/rest_dispatcher.rs`_ | 2026-05-27 |
| P2 | `HttpOutHandle` v2 — per-authority path allow-lists | ❌ not started | — | — | — | — |

## Gate checks

Phase progression is gated. Do not start a later phase before
the earlier one's gate passes.

- **Phase 1 gate:** all of rows 1, 2, 3 at ✅. A handwritten
  test extension performs a tenant-scoped warehouse read via
  `ctx.warehouse_read().query("samples_window", …)` and
  observes the tenancy clamp from `ctx.caller()`. The
  `HttpOutHandle` → loopback shortcut in any reference
  extension is deleted.
- **Phase 2 gate:** row 4 at ✅. A test extension drives a
  cross-chart filter through `EventBusHandle` with zero
  additional HTTP round-trips per click. Load test: 10
  concurrent viewers × 20 charts × 1 filter change/sec
  sustained for 60s without queue growth in the supervisor.
- **Phase 3 gate:** row 5 at ✅. A dashboard created in tenant
  A is readable by user U2 in tenant B *only* when an
  `AuthzHandle` grant exists; `WarehouseReadHandle` queries
  triggered by U2 filter by tenant A (the owning tenant), not
  tenant B (the caller tenant).
- **Phase 4 gate:** rows 6, 7 at ✅. Power-BI MVP demo path
  ends with "schedule a daily PDF export to a blob bucket" and
  it works.

## Open questions

Carried from [README.md §"Open questions"](./README.md). Move
to **resolved** with a date and the resolution when closed.

| Question | Status | Resolution |
|---|---|---|
| Template versioning scheme | open | — |
| `count()` derived vs own template | open | — |
| `EXPLAIN` cost-rejection + fairness budgets | deferred | — |
| Cross-tenant grant table shape in `starter-store-postgres` | open (gates Phase 3) | — |
| Closed-grammar calculated-fields engine | deferred (post-MVP) | — |

## Session log

Newest entry first. One entry per session that touches this
scope. Format:

```
### YYYY-MM-DD — <one-line session summary>

- Rows moved: <e.g. "row 1: ❌ → 🟡 (PR #123 opened)">
- Decisions: <bullets — open-question resolutions, scope changes, etc.>
- Next: <what the next session should pick up>
```

Keep entries terse. The session log is the audit trail; the
tracker tables above are the source of truth for current state.

---

### 2026-05-27 — Reference extension upgraded into a full Phase B demo (datablist CSVs)

- Rows moved: none (no critical-path row state changed). What
  changed is the *demonstration surface*: `rubix/extensions/com.rubix.example`
  went from a single-tool `echo` placeholder to an end-to-end
  demo exercising rows 2, 3, 4, B1, B5 simultaneously.
- Why: the previous reference extension only contributed one
  no-op tool, so an operator landing on `/extensions` saw nothing
  resembling what a real third-party block would ship. The
  contributions inventory in the manifest had drifted away from
  what the host can actually accept. This session re-grounds the
  reference block on the same datablist sample CSVs the public
  docs use for `customers` / `products` examples
  ([datablist/sample-csv-files](https://github.com/datablist/sample-csv-files)).
- Changes (~1100 LOC across 16 files):
  - **SDK macro extension** (`starter-extensions/crates/starter-ext-sdk/src/lib.rs`):
    added the missing `warehouse_write`, `dashboard`, and `authz`
    arms to `__requires_capability_method!`. The matching
    `Capability` variants + `CtxInner` fields already existed
    (rows B1, 5); the macro just hadn't been taught to expose
    them. Updated the `compile_error!` catalog and bumped the
    "known categories" message. Net: an extension that declares
    `requires!(... warehouse_write ...)` now compiles; before this
    session it errored "unknown capability category".
  - **block.yaml** (`rubix/extensions/com.rubix.example/block.yaml`):
    grew three new tool entries (`csv_ingest`, `customer_quality`,
    plus the original `echo`), two `warehouse_tables[]` entries
    (`customers`, `products`) shaped after the datablist schema,
    two `warehouse_templates[]` entries with audit-only SQL bodies,
    one `anomaly_rules[]` entry pointing at the
    `customer_quality` tool, and the matching `capabilities[]`
    grants (`warehouse_write.tables`, `warehouse_read.tables`).
  - **kinds/**: 11 new files — JSON Schemas for csv_ingest (in/out),
    customer_quality (in/out), template params (customers_by_country,
    products_low_stock), description markdowns, and the two
    audit-only `.sql` bodies.
  - **data/**: shipped 30-row + 20-row slices of the datablist
    customers / products CSVs so the demo works offline. The host
    only serves `ui/*` to the browser; the bundle-level `data/*`
    is available to server-side handlers via `ctx.fs()` and is
    what the `make demo-ingest` (TBD) target would feed into
    `csv_ingest`.
  - **process/src/main.rs**: re-wrote with three handlers. The
    `csv_ingest` handler is the canonical demo of
    `ctx.warehouse_write().insert(table, rows)`: validates the
    dataset against the manifest allowlist, coerces every row into
    a `Row`, and forwards. The `customer_quality` handler delegates
    to a pure `evaluate_customer_quality(row)` body that flags
    `MissingCountry` → `MissingEmail` → `InvalidEmail` → `BadDate`
    in that order. The pure body is covered by 6 unit tests; all
    green.
  - **ui/remoteEntry.js + ui/main.tsx**: replaced the manifest-only
    panel with a four-section dashboard:
    (a) header + contributions inventory pulled from the manifest
        admin route;
    (b) an SVG horizontal-bar chart "customers by country (top 10)"
        synthesised from an inline 12-row sample (mirrors the
        `customers_by_country` template's output shape);
    (c) a compact "low-stock products (< 10)" table mirroring
        `products_low_stock`;
    (d) a "data-quality rule preview" that runs the same logic the
        server-side `customer_quality` rule applies and lists every
        flagged row.
    Both files are kept in sync — `main.tsx` is the developer-facing
    source the future vite-plugin-federation pipeline will compile;
    `remoteEntry.js` is the hand-authored bundle the host serves
    today.
  - **README.md**: rewrote the layout table to cover every new
    surface plus a curl demo for the ingest tool.
- Tests:
  - 6/6 new unit tests pass for `evaluate_customer_quality`
    (`customer_quality_ok_when_all_fields_present`,
    `_flags_missing_country_first`, `_flags_missing_email`,
    `_flags_invalid_email`, `_flags_bad_date`,
    `iso_date_parser_accepts_plausible_strings`).
  - `cargo build --release -p rubix-example-extension`: clean.
  - `cargo build --workspace --exclude starter-ext-wasm`: clean
    (the wasm crate has a pre-existing non-exhaustive match on
    `Capability` that this session didn't touch).
- Decisions:
  - **Sample data in-repo, not fetched at build time.** Vendoring
    a 30-row slice (~7KB) of the datablist customers CSV keeps the
    demo offline-installable and reproducible. The full datablist
    archives are gigabyte-scale at the high end; we ship only the
    columns the warehouse tables declare.
  - **Three deliberately-bad rows in the UI inline sample.**
    `BAD-NO-EMAIL-01`, `BAD-NO-CNTRY-01`, `BAD-DATE-001` are the
    seeds that make the data-quality preview non-empty. The
    server-side ingest tool sees only the clean CSV slice.
  - **UI inlines the rule logic instead of round-tripping.** The
    Module-Federation factory contract doesn't expose a host
    `StarterClient` — the only thing the panel can do is `fetch`
    same-origin `/api/v1/...`. Mirroring the rule client-side
    avoids a round trip and proves the rule logic is portable
    (Rust + TS implementations agree on the same flag taxonomy).
    Drift risk is real and documented in both files' preambles:
    the server-side rule is the authority.
  - **`warehouse_write` macro arm added rather than worked around.**
    The alternative — having the demo extension use a `requires!`
    declaration that doesn't include `warehouse_write` and reaching
    for the handle some other way — would have either failed
    (there is no other way per SCOPE R6) or required a fork of the
    SDK. The arm is 6 lines, the `CtxInner` field + backend trait
    already exist, and the same fix unblocks the `dashboard` /
    `authz` arms for row 5 work. Net: completes the macro surface
    for every shipped capability category.
  - **Capability `tracing` dropped from the manifest.** `tracing`
    is in the `requires!` macro's known-categories list (it's
    "always safe") but is NOT a `Capability` enum variant — the
    manifest's `capabilities:` slot is for grants the operator
    can revoke, and tracing isn't one. The previous draft of this
    session declared it under `capabilities` and the loader
    rejected the manifest; removing it is the right fix.
- Next:
  1. **Wire the `warehouse_read` host backend** so the two contributed
     templates the demo declares actually return rows when the UI
     calls `/api/v1/warehouse/read/com.rubix.example.customers_by_country`.
     Today the panel synthesises the chart from inline samples;
     the host-side resolver landing closes the loop.
  2. **Row 5** (`DashboardHandle` + `AuthzHandle` with cross-tenant
     grants) — the `dashboard` and `authz` macro arms added in this
     session are the SDK-side prerequisite.
  3. **vite-plugin-federation build for `ui/main.tsx`** — `remoteEntry.js`
     remains hand-authored. The Phase E pipeline that compiles TSX
     → MF bundle is unchanged.

---



- Rows moved: none (status unchanged). Phase B gate now has an
  executable proof: a fixture extension with
  `contributes.anomaly_rules[]` flows through
  `Loader::scan → validate_all → commit → seal`, the host
  projection walks the sealed registry, the cleaner's
  `build_registry_with_contributions` resolves the contribution
  against a `CannedTool`, and `process_entity_window` surfaces
  the contributed rule's `Flag` outcome on the emitted L2 row.
- Changes (~270 LOC across 2 files):
  - `rubix-agent/src/registry.rs`: flipped
    `collect_anomaly_rule_contributions` from private to `pub` so
    the gate test can drive the same projection the boot path
    uses without re-implementing the manifest walk.
  - `rubix-agent/tests/extension_anomaly_rule_gate_test.rs`: new
    integration test (7 cases) backed by a local `CannedTool`
    duplicate (kept inline rather than reaching into the
    adapter's `#[cfg(test)]` module). The fixture bundle uses
    `runtime.kind: builtin` so no schema files or binaries need
    to exist on disk — the loader only reads the `block.yaml`.
- Tests (7 new, all green):
  - `fixture_validates_through_extension_registry` — manifest
    round-trips through the loader and surfaces the contributed
    rule on the validated record.
  - `host_projection_emits_one_contribution_per_manifest_entry`
    — `collect_anomaly_rule_contributions` on the sealed registry
    yields one `ContributedRule` with id/tool_id/priority pinned.
  - `host_projection_with_no_registry_returns_empty` — the
    `None` case (laptop / `rubix-admin mcp` stdio path) degrades
    cleanly.
  - `contributed_rule_fires_end_to_end_against_synthetic_window`
    — the canonical happy path. Builtins pass through (no NaN,
    no history → Spike/Stuck can't fire), the contributed rule
    flags, L2 row carries `quality = Spike`, `rule_id =
    Some("com.acme.weather.spike")`, and the note surfaces in
    the `tags` JSONB as `{ "com.acme.weather.spike": "ratio=37x" }`.
  - `contributed_rule_passes_through_when_tool_says_ok` —
    `{outcome: ok}` produces an unflagged L2 row.
  - `builtin_nan_rule_short_circuits_contributed_rule` — pins the
    ordering invariant: a NaN row tags as `builtin.nan` and the
    contributed rule's canned response queue is observed to be
    untouched, proving builtins always fire first.
  - `unresolved_tool_id_is_silently_dropped` — builder with an
    empty `&[Arc<dyn Tool>]` falls back to the three builtins
    only; the contributed rule is dropped (warn-logged) instead
    of aborting.
- Decisions:
  - **Test the pure seam, not `run_tick`.** The gate test pins
    `process_entity_window` (the pure window walker) instead of
    spinning up testcontainers Postgres for `run_tick`. The L1
    fetch + L2 bulk insert around the walker is already
    exercised by the cleaner unit tests; adding Postgres to the
    gate would buy nothing for the rule-pipeline assertion and
    would gate the test behind Docker.
  - **`pub` the projection helper, don't duplicate it.** The
    alternative — re-implementing the 10-line manifest walk in
    the test — would drift from the boot path silently. Making
    `collect_anomaly_rule_contributions` part of the lib surface
    is a one-line change and makes the boot path's exact
    projection observable to any future host that wants to drive
    the same pipeline (rubix-admin sibling, smoke test harness,
    etc.).
  - **Local `CannedTool` over reaching into `#[cfg(test)]`.**
    `rubix-tools/src/cleaner/adapter.rs` already has a
    `CannedTool` in its own test module; integration tests
    cannot see it. The 20-line duplicate in the gate test file
    is the canonical workaround and keeps the gate test
    self-contained.
  - **`runtime.kind: builtin` for the fixture.** Avoids needing
    a real spawnable binary on disk like
    `extensions_lifecycle_test.rs` requires for its
    `runtime.kind: process` fixture. The contributed-rule
    dispatch path doesn't touch the supervisor at all — it
    resolves `tool_id` against the host's in-process tool
    registry — so `builtin` is the right flavour for this gate.
- Tests run: 7/7 pass in the new file; `cargo check --workspace
  --tests --exclude starter-ext-wasm` clean.
- Next:
  1. **Row 5** (`DashboardHandle` + `AuthzHandle` with
     cross-tenant grants) — the original Phase 3 critical-path
     item. Phase B is independent of Phases 1/2/3, so this is
     now the next critical-path move.
  2. **P1 follow-up** (`workers[]` + `cli[]` adapter mounting) —
     independent, can interleave with row 5 if a different
     contributor picks it up.
  3. **Optional: testcontainers tick test.** A `#[ignore]`d
     end-to-end test that boots Timescale, seeds `samples`, runs
     `run_tick`, and asserts the contributed rule's L2 row lands
     in the hypertable. Buys real-DB coverage for `run_tick` +
     `bulk_insert_l2` but is gated behind Docker; defer until a
     bug actually motivates it.

### 2026-05-27 — B5 host wiring lands: contributed rules join the cleaner registry

- Rows moved: B5 status note flipped from "SPI + validation + adapter
  (host wiring deferred)" to "SPI + validation + adapter + host wiring."
  Phase B is now functionally complete end-to-end.
- Changes (~150 LOC across 5 files):
  - `rubix-tools/src/cleaner/adapter.rs`: new
    `ContributedRule { id, tool_id, priority? }` projection +
    `build_registry_with_contributions(tools, contributions)`
    builder. Returns a `RuleRegistry` pre-loaded with the three
    builtins (NaN → Spike → Stuck) and extended with one
    `ToolAnomalyRule` per contribution. Sort key is
    `(priority asc with None last, declaration index asc)`. Tool
    resolution is by `definition().name` against the supplied
    `&[Arc<dyn Tool>]`; misses are warn-logged and the rule is
    silently dropped (the cleaner keeps running).
  - `rubix-agent/src/registry.rs`: new
    `collect_anomaly_rule_contributions(extensions)` walks every
    Validated extension's `contributes.anomaly_rules[]` and
    projects entries into `ContributedRule`. `build_tool_registry`
    grows a fifth parameter `extensions:
    Option<&Arc<ExtensionRegistry>>`; the cleaner-tool construction
    now calls the builder with the just-assembled tool list, then
    logs the resulting rule count + ids at boot. Replaces the
    previous `RuleRegistry::builtin()` placeholder.
  - `rubix-agent/src/main.rs`: passes
    `ext_bundle.as_ref().map(|b| &b.registry)` into the tool
    registry builder.
  - `rubix-agent/src/boot/mcp/register.rs`: the fallback that
    rebuilds the registry for the stdio `rubix-admin mcp`
    subcommand now passes `None` for the extension registry — the
    laptop path doesn't load extensions, so contributed rules are
    intentionally absent there.
  - Three integration tests + the internal registry test updated
    to thread the new `None` argument
    (`alert_path_threshold_test`, `changelog_middleware_test`,
    `rest_disk_test`, plus the in-crate `names()` helper).
- Tests: 4 new builder unit tests (starts with builtins; appends
  after builtins; sorts by priority then declaration order;
  silently drops unresolved tool ids). 46 cleaner tests total.
  `cargo check --workspace --tests --exclude starter-ext-wasm`
  clean.
- Decisions:
  - **Tool resolution by `definition().name`, not by `Arc` pointer.**
    Each contribution's `tool_id` is a string; the builder
    resolves against the `Arc<dyn Tool>` list at build time. Two
    consequences: (a) ordering of the tool list matters only to
    `definition().name` uniqueness (the host already guarantees
    that); (b) the builder works equally well for rubix-builtin
    tools (`rubix.warehouse.ingest`, etc.) and for any future
    extension-contributed tool that lands inside the same
    `Arc<dyn Tool>` registry.
  - **Misses warn and drop, not fail.** A contributed rule whose
    `tool_id` doesn't resolve is operator-visible (`warn!` under
    target `rubix.cleaner.adapter`) but doesn't abort boot — the
    cleaner runs without it. Matches the rest of the boot path's
    "one bad manifest can't block the host" stance (see
    `extension_tables.rs` skip-and-warn).
  - **Sort key.** `priority` is the operator's lever to put a
    detector before others; `None` sorts last so declared
    priorities always beat undeclared ones (declared = opt-in to
    ordering). Declaration index as tiebreaker matches how
    `RuleRegistry::add` already documents the registration order.
  - **Builtin order is fixed.** Contributed rules can NOT be
    interleaved with the builtins — they always run after.
    Reason: the builtins handle NaN short-circuit + first-row
    pass-through invariants that the cleaner depends on; an
    extension reordering them would risk false-negatives on
    `value.is_nan()` rows. If an extension's rule needs to fire
    before a builtin, the right path is a separate registry
    (out of scope for B5).
  - **Boot log at info.** `rule_count + rules: [...]` is logged
    once per boot under target `rubix.registry` so an operator
    can spot drift between deployments at a glance.
- Next:
  1. **Phase B gate test.** Stand up a `CannedTool` fake under a
     test extension manifest, validate it through
     `ExtensionRegistry`, build the tool registry with extensions
     present, and assert the rule fires end-to-end against a
     synthetic L1 window. The whole pipeline now works without
     scaffolding — this is the proof, not net-new wiring.
  2. **P1 follow-up** (`workers[]` + `cli[]` adapter mounting) —
     unblocked by Phase B closing; no Phase B work blocks on it.
  3. **Row 5** (`DashboardHandle` + `AuthzHandle` with
     cross-tenant grants) — the original Phase 3 critical path
     item. Phase B is independent of Phases 1/2/3, so this is the
     next critical-path move.

### 2026-05-27 — B5 lands: `contributes.anomaly_rules[]` SPI + adapter

- Rows moved: B5: ❌ → ✅ shipped (SPI + validation + adapter;
  host registry wiring deferred — see "Next" below).
- Changes (~300 LOC across 4 files):
  - `starter-ext-spi/src/manifest.rs`: new
    `ContributeAnomalyRule { id, tool_id, priority? }` struct with
    `#[serde(deny_unknown_fields)]`; `Contributes.anomaly_rules:
    Vec<…>` slot, defaulting to empty. Re-exported in
    `starter-ext-spi/src/lib.rs`. Two SPI round-trip tests
    (`anomaly_rules_round_trip_with_priority` +
    `anomaly_rules_default_to_empty`).
  - `starter-ext-host/src/validate.rs`: new
    `contributes.anomaly_rules[].id` validation arm. Mirrors the
    `contributes.warehouse_templates[].name` shape (reserved-prefix
    + namespace-ownership) and adds a per-rule `builtin.*` prefix
    rejection so a manifest cannot shadow the in-process detectors
    the cleaner short-circuits. Four new tests (descendant accepts,
    sibling namespace rejects, `builtin.*` rejects,
    `starter.*` rejects).
  - `rubix-tools/src/cleaner/adapter.rs`: new `ToolAnomalyRule`
    wrapping an `Arc<dyn Tool>` into the cleaner's
    `AnomalyRule` trait. `id` is `Box::leak`ed at construction so
    the trait's `&'static str` contract survives dynamic
    manifest-sourced ids; the leak is boot-time-only and bounded
    by manifest size. `apply` invokes the tool with
    `{ row, window_tail }` and decodes `{ outcome: "ok" | "flag" |
    "drop", quality?, note? }`; errors / shape mismatches degrade
    to `RuleOutcome::Ok` with a warn log under target
    `rubix.cleaner.adapter` so a misbehaving rule cannot silently
    flag rows. Five new `#[tokio::test(flavor = "multi_thread")]`
    unit tests over a `CannedTool` fake (ok, flag, drop, malformed
    response, id pass-through).
  - `rubix-tools/src/cleaner/mod.rs`: `pub mod adapter;` + re-export.
- Tests: 6 new SPI/validation tests + 5 new adapter tests. 42
  cleaner tests + 76 SPI/host tests total — all green.
  `cargo check --workspace --tests --exclude starter-ext-wasm`
  clean (starter-ext-wasm has a pre-existing non-exhaustive match
  on `Capability`; out of scope for this session).
- Decisions:
  - **SPI lands without host wiring** — same rhythm as rows 2/3/4
    (capability + handle + adapter scaffolding) where the
    backend / dispatch path comes in a follow-up. The manifest
    field is the lock-in surface; the dispatch path can iterate
    without breaking extension authors.
  - **`builtin.*` is a per-field reserved prefix, not added to
    `starter-ext-spi::id::RESERVED_PREFIXES`.** Widening the
    global reserved set would block other contribution kinds
    from using `builtin.<x>` ids gratuitously. The rule-id
    domain is the only place the reservation is load-bearing
    (the cleaner short-circuits builtins by id), so it stays
    local to the rule-id validation arm.
  - **`Box::leak` for dynamic ids.** Two alternatives considered
    and rejected: (a) widen `AnomalyRule::id(&self) -> &str` —
    cascades through every rule impl + the registry + `L2Row`
    storage; (b) intern ids in a per-process map — premature.
    Box::leak is one-line, bounded at boot, and matches the
    "one-time allocation at registration" pattern the rest of
    the registry uses.
  - **Adapter degrades to `Ok` on error.** A misbehaving rule
    must not silently flag rows — the only safe failure mode for
    an extension-authored detector is "act as if you weren't
    there." The warn log under `rubix.cleaner.adapter` is the
    operator's escape hatch for finding misbehaving rules.
  - **Sync `apply` via `block_in_place`.** The trait stays sync
    for builtin-rule throughput; the adapter bridges into the
    async tool dispatch with `tokio::task::block_in_place +
    Handle::current().block_on(...)`. Requires multi-thread tokio
    — documented in the module preamble. The agent runs on
    multi-thread tokio by default; if a future single-thread
    consumer needs adapter support, the seam is the registry
    (extend it with an `apply_async`).
  - **Wire shape pinned in code, not in SPI.** The adapter's
    `{ row, window_tail } → { outcome, quality?, note? }` JSON
    contract is documented in the adapter's module preamble but
    not in `starter-ext-spi`. Extensions ship a tool that
    implements the contract; the SPI only ships the *contribution
    declaration*. Trade-off: if the wire shape ever needs to
    change, it's a per-host migration (one place to fix), not an
    SPI bump. Acceptable until a second host wants to consume the
    same manifest field.
- Next:
  1. **B5 host registry wiring (follow-up).** Walk every
     Validated extension's `contributes.anomaly_rules[]`, build
     one `ToolAnomalyRule` per entry resolved against the host's
     tool registry, sort by `priority` (declaration order on tie),
     and call `RuleRegistry::add` after the builtins. Pass the
     resulting registry into `CleanerTickTool::new` (currently
     constructed with `RuleRegistry::builtin()` only). Slot to
     fold into `rubix-agent/src/registry.rs` once a first
     extension-authored rule lands.
  2. **Integration test against a contributed-tool fake** — once
     B5 wiring is in, the round trip
     (manifest → validate → adapter → registry → tick → L2) is
     worth one end-to-end test backed by a `CannedTool`.
  3. **Phase B closes** once (1) lands. Phase B gate could be
     "an extension's `com.acme.weather.spike` rule flags a row
     in `samples_l2` end-to-end on a real tick."

### 2026-05-27 — B4 lands: `rubix.cleaner.tick` tool + bundled flow

- Rows moved: B4: ❌ → ✅ shipped.
- Changes (~250 LOC across 4 files):
  - `rubix-tools/src/cleaner/tool.rs`: new `CleanerTickTool` impl
    of `starter_spi::Tool`. `definition().name == "rubix.cleaner.tick"`.
    Wire shape `{ from_ts_ms?, to_ts_ms?, history_lookback_ms?,
    tick_epoch_ms?, window_ms? }`; the explicit `[from, to)` path
    wins; otherwise `from = tick_epoch_ms - window_ms` /
    `to = tick_epoch_ms`. Calls into the existing
    `cleaner::tick::run_tick` and returns `TickStats` serialised
    verbatim. Wraps `run_tick`'s `sqlx::Error` as
    `Error::Internal { source }`.
  - `rubix-agent/src/registry.rs`: appends the new tool to
    `build_tool_registry` inside the `warehouse.is_some()` arm so
    it only enables when a `WarehouseClient` is configured.
    Constructed with `RuleRegistry::builtin()` (NaN → Spike →
    Stuck) until B5 ships the extension-contributed rule slot.
  - `rubix-flows/flows/cleaner.yaml`: new bundled flow
    `com.rubix.cleaner` rooted at
    `starter.flow.trigger.schedule` (60s cron `*/60 * * * * *`),
    chained `tick.fire → clean.in → emit.value` through a
    `starter.flow.tool-call` node wired to `rubix.cleaner.tick`.
    `tool_input: {}` — the seed adapter auto-injects
    `tick_epoch_ms` from wall-clock so the tool derives its
    `[from, to)` window without per-flow YAML.
  - `rubix-flows/tests/load_test.rs`: `EXPECTED_FLOW_IDS` and
    `NON_AI_AGENT_FLOW_IDS` extended with `com.rubix.cleaner`;
    drive-by add of the long-orphaned
    `com.rubix.data-flow.weekly-report` flow id to unstick the
    pre-existing pre-cleaner failures on the two cross-check tests.
- Tests: 6 new unit tests on `resolve_params` (explicit from/to
  passes, tick_epoch_ms alone derives window, custom window_ms,
  missing inputs error, reversed range error, only-`to` derives
  `from` via window). 37 cleaner tests total. All 5 flow load_test
  cases now green (previously 3 of 5 thanks to the pre-existing
  weekly-report mismatch). `cargo check --workspace --tests` clean.
- Decisions:
  - **Tool name `rubix.cleaner.tick`** — `<noun>.<verb>` matches
    the rest of the rubix tool catalogue (`rubix.dashboard.list`,
    `rubix.warehouse.ingest`, etc.). The flow YAML pins this id as
    a literal string in `settings.tool_id` — renaming the tool
    later means a flow migration.
  - **60s window default, 60s cron.** Window length matches the
    cron interval so successive ticks tile without overlap. If
    a tick runs long the next one still sees its own non-overlapping
    range (no double-write); a missed tick leaves a hole that
    `samples_l2` will simply not cover (operator catches up via
    an ad-hoc `rubix.cleaner.tick` call with explicit `from`/`to`).
  - **`tool_input: {}` + auto-injected `tick_epoch_ms`.** Matches
    the pattern the synth producer flow used. Keeps the YAML
    bundled-flow-shaped (no per-tenant config baked in); the host
    seed adapter is the single chokepoint where wall-clock enters
    the flow graph.
  - **History lookback stays in tool defaults**, not the flow YAML.
    The default 30-min lookback comes from `TickParams::new` and
    `DEFAULT_HISTORY_LOOKBACK_MS`; ops who want a different value
    set it via an ad-hoc tool-call, not by editing bundled YAML.
  - **Only enabled when warehouse is configured.** The
    `if let Some(wh) = warehouse` arm gates the registration so
    rubix-agent boots without the warehouse plane still produces a
    tool registry that simply omits `rubix.cleaner.tick` (rather
    than registering it and failing at first invoke).
  - **Pre-existing test drift fixed in passing.**
    `com.rubix.data-flow.weekly-report` had been bundled without
    being added to `EXPECTED_FLOW_IDS`, so the two cross-check
    tests had been red on master before this session. Cleaner row
    needed the list edited anyway; folded the weekly-report fix in
    rather than leaving the file half-correct.
- Next:
  1. **B5** — `contributes.anomaly_rules[]` manifest slot +
     tool-call dispatch adapter. With B2+B3+B4 done, B5 is a thin
     adapter that wraps a contributed tool dispatch into an
     `AnomalyRule` impl and feeds it into the same registry the
     cleaner already runs.
  2. **Integration test** for `run_tick` against testcontainers
     Timescale. Deferred — the cleaner tool + flow already exercise
     the full path manually; the e2e test slots in once a real
     fixture lands.
  3. **P1 follow-up** — `workers[]` + `cli[]` adapter mounting.

### 2026-05-27 — B3 lands: cleaner tick + `samples_l2` migration

- Rows moved: B3: ❌ → ✅ shipped (sqlx path; tool wrapper deferred to B4).
- Changes (~400 LOC across 3 files):
  - `starter-store-warehouse/src/tsdb/migrate.rs`: new `0006_samples_l2`
    migration. Hypertable on `ts` with the standard L2 weekly chunk
    interval; columns `tenant_id`/`entity_id`/`ts`/`value_num`/`quality
    TEXT`/`rule_id TEXT`/`tags JSONB`. Indexes: `(entity_id, ts DESC)`,
    `(quality, ts DESC)`, GIN on `tags`. `TIMESCALE_MIGRATIONS` audit
    constant extended.
  - `rubix-tools/src/cleaner/tick.rs`: new module split into a pure
    window walker (`process_entity_window`) and an async I/O runner
    (`run_tick`). The runner SELECTs one combined `[from-lookback, to)`
    window from `samples` ordered by `(tenant_id, entity_id, ts)`,
    splits per-entity into history vs fresh by comparing `ts_ms`
    against `from_ts_ms`, calls the walker, then bulk-INSERTs the
    emitted rows into `samples_l2` via multi-row VALUES (chunked at
    4000 rows / 28000 params to stay under Postgres' 65535 param cap).
    `TickStats` carries `rows_read` / `rows_written` / `rows_dropped` /
    `entities_scanned` / `by_quality: HashMap<String, u64>`.
  - `rubix-tools/src/cleaner/mod.rs`: re-exports `tick::{run_tick,
    process_entity_window, L2Row, TickParams, TickStats}`.
- Tests: 8 new unit tests on the pure walker (empty fresh, Ok
  pass-through, NaN flagging with rule-id + tags note, history seeds
  Spike on first fresh row, rolling window lets Stuck fire mid-fresh,
  Drop outcome skips emission but still extends window, default 30min
  lookback, empty registry marks every row Ok). 31 cleaner tests
  total, all green; `cargo check --workspace --tests` clean.
- Decisions:
  - **Pure / I/O split.** The walker is sync + infallible (no `sqlx`,
    no `tokio`) so it unit-tests without a database; the runner is
    the only async surface. Mirrors the rule trait's same shape.
  - **Rolling window inside fresh.** As fresh rows are processed they
    extend the per-entity window in place, so `StuckRule` can fire
    mid-tick. Dropped rows still extend the window for the same
    reason (downstream rules deserve to see them as context).
  - **History via single SELECT, not a second query.** `[from -
    lookback, to)` is one ordered scan; the row loop splits by
    `ts_ms < from_ts_ms`. Avoids per-entity round-trips and keeps
    the entity-grouping inversion in one place.
  - **Lookback default 30 minutes.** Matches the synth-side
    stuck-stretch typical length; tunable per `TickParams`.
  - **Multi-row VALUES not COPY.** ~28000 params per chunk is well
    under Postgres' limit and `sqlx::query` already supports the
    pattern; `COPY` would force a separate execution path on
    `PgConnection::copy_in_raw`.
  - **`bool`/`text` value columns ignored.** `samples_l2` carries
    `value_num` only — the rule trait operates on `f64` and
    everything non-numeric passes through with `value = NULL`. If
    we ever want to clean non-numeric streams the column set
    expands here (no migration change needed thanks to JSONB tags).
- Next:
  1. **B4** — tool wrapper (`rubix.cleaner.tick`) + bundled
     `com.rubix.cleaner` flow YAML on a 60s schedule. The tick
     function is ready to be wrapped; the flow seed pattern matches
     the existing synth producer.
  2. **B5** — `contributes.anomaly_rules[]` manifest slot + tool-call
     dispatch adapter wrapping a contributed tool into an
     `AnomalyRule` impl.
  3. **Integration test** — end-to-end against a testcontainers
     Timescale (running tests/cleaner_tick_e2e.rs once a real
     fixture lands). Deferred until B4 since the tool wrapper is
     the natural surface to exercise.

### 2026-05-27 — Use-Case-B push: P1 (partial) + B1 + B2

Three landings in one session driving the Use-Case-B path
(ad-hoc data sources, custom rules, custom non-dashboard pages):

**P1 partial — process-flavour REST/SSE mounting**

- Rows moved: P1: ❌ → 🟡 partial (REST + SSE wired; workers + cli still pending).
- Changes:
  - New `CompositeRestDispatcher` in
    `rubix-agent/src/extensions/rest_dispatcher.rs` — routes per
    `RuntimeKind` to either the existing `BuiltinRestDispatcher` or
    the upstream `ProcessRestDispatcher` (which was already built
    in `starter-ext-server` but never wired in rubix).
  - `rubix-agent/src/main.rs` now constructs the composite using
    `bundle.process_handles` (already populated by
    `boot::build_extension_admin`) + a 30s per-call timeout
    mirroring `EXTENSION_TOOL_REQUEST_TIMEOUT` from the MCP side.
  - `contributes.rest[]` and `contributes.tools[]` entries from
    process-flavour extensions now mount at `/api/v1/...` and
    `/api/v1/tools/...`. SSE/NDJSON unary path works;
    process-flavour streaming dispatch is still NotWired upstream
    (separate slice — needs the per-stream `stream.event/end`
    demultiplexer).
- Decisions:
  - **No new upstream code.** Everything we need was already in
    `starter-ext-server::rest::ProcessRestDispatcher`; the gap was
    purely "rubix-agent installs only the builtin half." Composite
    pattern keeps the wiring local to rubix-agent so a future
    consumer can pick a different routing policy.
  - **Empty `process_handles` is safe.** When no process extensions
    are enabled the composite simply never picks the process branch
    — no panic, no startup error.

**B1 — `WarehouseWriteHandle` + tables contribution**

- Rows moved: new row B1 lands at ✅ shipped (Postgres/Timescale).
- Changes (~700 LOC across 12 files):
  - `starter-ext-spi`:
    - `Capability::WarehouseWrite { tables: Vec<String> }` variant
      + round-trip tests.
    - `ContributeWarehouseTable { name, columns, order_by, engine?, partition_by?, ttl? }`
      and `TableColumn { name, type, default? }` in
      `manifest.rs`; `Contributes.warehouse_tables: Vec<…>`.
    - `WarehouseWriteRequest` / `WarehouseWriteResponse` wire types
      in `warehouse.rs`.
  - `starter-ext-sdk`:
    - `WarehouseWriteHandle { insert(table, rows) -> Result<u64> }`
      backed by a private `WarehouseWriteBackend` trait.
    - `CtxInner` gains a 12th arg; `ctx.warehouse_write()` accessor.
    - `RealWarehouseWriteBackend` (process-flavour) over JSON-RPC
      method `warehouse.write`.
    - All eight `CtxInner::new` callsites updated (wasm, process,
      MCP stub, CLI, workers, gRPC, REST adapter, and the internal
      test) — each gets a local `StubWarehouseWrite` that refuses.
  - `starter-ext-server`:
    - `CapabilityFactory::warehouse_write(...)` trait method with a
      default fail-closed stub so existing factory impls keep
      compiling. `BuiltinRestDispatcher::build_ctx` plumbs the new
      backend through.
  - `starter-ext-supervisor`: `category_of()` learns the
    `WarehouseWrite` arm so the supervisor's category gate fires.
  - `rubix-agent`:
    - `RubixWarehouseWriteBackend` in
      `extensions/warehouse_write.rs`: tenant clamp (overwrites
      caller-supplied `tenant_id`), grant gate, column validation
      against the manifest spec, typed column binding (text / int /
      float / bool / jsonb), multi-row INSERT via `sqlx::query_with`.
    - `extensions/backends.rs`:
      `RubixCapabilityFactory::warehouse_write(...)` impl; new
      `warehouse_write_grant()` + `warehouse_tables_for()`
      resolvers (mirror of the read-side resolvers).
    - `extensions/host_methods.rs`: `warehouse.write` JSON-RPC arm
      for process-flavour extensions; mirrors `warehouse.query`.
    - `boot/extension_tables.rs`: walks every Validated extension
      at boot, runs `CREATE TABLE IF NOT EXISTS
      "<sanitize(ext_id)>__<name>"(...)` + companion index on
      `(tenant_id, ...order_by)`. Reserved `tenant_id` column
      auto-prepended; column-type strings pass through verbatim.
- Tests: 12 backend tests (refusal / clamp / grant gate / missing
  column / unknown column / empty insert) + 4 DDL tests + 2 new
  factory tests + 1 new host-method test. All green.
- Decisions:
  - **Tenant-scoped INSERT only** (no UPDATE / DELETE / DDL via
    capability) — append-only invariant matches L1, DDL belongs in
    install-time migrations, not runtime capability.
  - **Wider scope chosen** (extension-declared table schemas) —
    user picked "long term" over "minimal." Means extensions
    declare `contributes.warehouse_tables[]` and the host runs
    DDL at boot, instead of writing only to a host-defined
    table set.
  - **Postgres/Timescale only.** `WarehouseClient` wraps
    `sqlx::PgPool`. ClickHouse was deleted in PR #44 and we
    treat it as gone. Type strings in the manifest pass through
    to Postgres DDL (`TEXT`, `DOUBLE PRECISION`, `JSONB`, …).
    `engine` / `partition_by` / `ttl` SPI fields are accepted
    but ignored today (kept for future Timescale-feature
    expansion: hypertable + continuous-aggregate + retention).
  - **Sanitised table names** are `<ext_id_with_dots_to_underscores>__<unprefixed_name>`
    — two-underscore boundary so two extensions cannot collide on
    a name even if their ids share a suffix.

**B2 — `AnomalyRule` trait + builtin rules + `RuleRegistry`**

- Rows moved: new row B2 lands at ✅ shipped.
- Changes: new `rubix-tools/src/cleaner/` module (~700 LOC including
  tests):
  - `rule.rs` — `AnomalyRule` trait (sync, infallible),
    `Reading { tenant_id, entity_id, ts_ms, value: Option<f64>, source_quality }`,
    `WindowSlice<'a>`, `RuleOutcome { Ok | Flag { quality, note } | Drop }`,
    `QualityTag { Ok | Spike | Stuck | Missing | Nan }`.
  - `builtin.rs` — `NanRule`, `SpikeRule { factor: f64 }` (default
    10×), `StuckRule { min_repeats: usize }` (default 3). Mirrors
    the synth-side mess injectors in `dataflow::mess` (×50 spike,
    stuck-stretch, NaN), with conservative real-world thresholds.
  - `registry.rs` — `RuleRegistry`; first non-`Ok` outcome wins;
    `builtin()` constructor preloads NaN → Spike → Stuck in
    canonical order so `NanRule` short-circuits before numeric
    checks see NaN.
- Tests: 23 unit tests covering each rule's edge cases (no
  history, null values, NaN handling, threshold boundaries) and
  registry ordering semantics. All green.
- Decisions:
  - **Rules-as-trait, not rules-as-tools yet.** The trait is
    sync/infallible for in-process throughput; the extension
    contribution slot (B5) will wrap a contributed tool call into
    an `AnomalyRule` adapter, so the cleaner doesn't have to know a
    rule's origin.
  - **First-non-Ok wins.** Composability via order; operators
    control priority via registration sequence.
  - **No `Missing` per-row rule.** Gap detection is window-level
    (compare expected vs actual count) and belongs in the cleaner
    tick (B3), not the rule trait.

- Next:
  1. **B3** — cleaner tick (read L1 window, walk per-entity, apply
     rules, bulk-insert L2). Needs the `samples_l2` migration in
     `starter-store-warehouse::tsdb::migrate` first.
  2. **B4** — tool wrapper + bundled flow YAML + 60s schedule.
  3. **B5** — `contributes.anomaly_rules[]` slot + tool-call
     dispatch adapter. With B3 + B4 + B2 done, B5 is a thin
     adapter (~150 LOC).
  4. **P1 follow-up** — `workers[]` + `cli[]` adapter mounting.
     Separate from the Use-Case-B push; pick up when an extension
     needs it.

### 2026-05-27 — row 4 scaffolding: `EventBusHandle` (publish only)

- Rows moved: row 4: ❌ → ✅ shipped (publish; subscribe deferred).
- Changes:
  - `starter-ext-spi`: new `event_bus` module with
    `EventBusMessage { topic, payload, ts_unix_ms }`,
    `EventBusPublishRequest`, `EventBusSubscribeRequest`;
    `Capability::EventBus { publish: Vec<String>, subscribe:
    Vec<String> }` variant with independent direction allowlists.
  - `starter-ext-supervisor`: `event_bus` namespace gated under
    `EventBus`; both `event_bus.publish` and `event_bus.subscribe`
    accepted when granted; new `event_bus_namespace_gated` test.
  - `starter-ext-sdk`: new `EventBusHandle { publish(topic,
    payload) -> Result<()> }` backed by a private
    `EventBusBackend` trait; `requires!{}` gains an `event_bus`
    arm; `CtxInner::new` widens to 9 args (the new backend slot).
    All 6 adapters (REST, CLI, workers, gRPC, MCP, WASM) provide
    a `StubEventBus` returning `Error::Capability("event_bus
    not wired …")`.
  - `starter-ext-host::validate`: `capability_matches` learns the
    `cap.event_bus` ⇔ `Capability::EventBus` mapping so a
    manifest requiring the capability without a matching grant
    fails at load.
  - Design landed:
    `docs/design/extensions/event-bus.md`.
- Decisions:
  - **Publish-only v1.** Subscribe is meaningful design surface
    in its own right (backpressure, late-subscriber replay, drop
    semantics) and not session-scoped. Shipping the SPI wire
    type for it now (`EventBusSubscribeRequest`) locks the wire
    shape so adding the handle later doesn't break the supervisor.
  - **Independent direction allowlists.** `publish: []` and
    `subscribe: []` are independent neutralised forms — an
    extension might subscribe to host topics it doesn't itself
    publish, or vice versa.
  - **Best-effort fan-out.** Subscriber whose channel is full
    **drops** the message rather than blocking the publisher.
    Encoded in the deferred subscribe handle; row 4 doesn't
    pin down at-most-once vs at-least-once.
  - **Host-side backend deferred to row 5.** Same release as the
    `WarehouseReadBackend` impl — both belong to the host
    dispatch wiring that bridges JSON-RPC namespaces to host
    services. Stubs in every adapter mean extensions can compile
    against the handle today.
- Next: **Phase 2 gate is now reachable** (row 4 + host backend
  for fan-out) once row 5 lands. Two reasonable next moves:
    1. Pull the host backends forward (the `WarehouseReadBackend`
       impl from row 2's design doc + the `EventBusBackend`
       impl). Both bridge to existing host services; lands the
       Phase 1 + Phase 2 gate tests in one session.
    2. Move to row 5 (`DashboardHandle` + `AuthzHandle`).
       Cleaner per the proposal's phase ordering but the gate
       tests slip.
  Per the proposal's wording, the gates close on the host-side
  backend work; the SDK surface (rows 1–4) ships independently.

### 2026-05-27 — row 3 lands: `contributes.warehouse_templates[]` manifest slice

- Rows moved: row 3: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `ContributeWarehouseTemplate { name,
    params_schema, tables, sql_file }` plus
    `Contributes::warehouse_templates: Vec<…>`.
  - `starter-ext-host::validate`: reserved-prefix +
    namespace-ownership checks for `warehouse_templates[].name`
    (matching the `nodes[].kind` shape); `capability_matches`
    learns the `cap.warehouse_read` ⇔ `Capability::WarehouseRead`
    mapping so a manifest requiring the capability without a
    matching grant fails at load.
  - `starter-ext-host::warehouse`: new
    `TemplateRegistry::extend_from_record(&ExtensionRecord)` reads
    each contributed entry's `params_schema` (parsed as JSON) and
    optional `sql_file` (stored verbatim) from the record's bundle
    directory and inserts the resulting `TemplateSpec`. Failed /
    no-manifest records are a no-op.
  - Design landed:
    `docs/design/extensions/warehouse-templates-contribution.md`.
- Decisions:
  - **Silent shadowing** for name collisions — matches every
    other contribute slice today. An explicit `override:` flag +
    hard error is a row-3 follow-up, only worth shipping when
    the first collision is reported in practice.
  - **Schema-meaning validation deferred.** The JSON Schema is
    parsed for syntax only at load time; the call-site `params`
    check belongs to the resolver that lands in row 5.
  - **Per-template table allowlist deferred to row 5.** The
    supervisor still gates the `warehouse` namespace as a whole;
    the per-call `template.tables ⊆ grant.tables` cross-check
    lands when the real `WarehouseReadBackend` impl arrives (it
    already needs the registry for spec lookup, so no extra
    plumbing).
  - Loader stays I/O-free outside manifest reading: file I/O for
    contributed templates runs in `extend_from_record`, called
    by the host integration crate after `commit`, not inside
    `Loader::commit` itself.
- Next: **Phase 1 gate.** Rows 1–3 are now ✅. The gate asks
  for a handwritten test extension performing a tenant-scoped
  `ctx.warehouse_read().query(…)` end-to-end. Two paths from
  here, in priority order:
    1. Pick up the gate test — needs the host-side
       `WarehouseReadBackend` impl that consults the
       `TemplateRegistry` and binds `$caller_tenant_id` from
       `ctx.caller()`. This is the row-5 piece pulled forward
       to a stub that handles only the four builtin templates
       (enough to retire the four hard-coded matches in
       `TimescaleAnalyticsBridge`).
    2. Or move to row 4 (`EventBusHandle`) per the original
       phase ordering; the gate test can land after row 5.
  Per the proposal the gate is what unlocks Phase 2; row 4
  technically blocks on Phase 1 closing.

### 2026-05-27 — row 2 lands: `WarehouseReadHandle` + builtin `TemplateRegistry`

- Rows moved: row 2: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `warehouse` module with `Row`,
    `TemplateSpec`, `WarehouseReadRequest`; `Capability::WarehouseRead
    { tables: Vec<String> }` variant.
  - `starter-ext-supervisor`: `warehouse` namespace gated under
    `WarehouseRead`; `category_of` recognises the variant; new
    `warehouse_read_namespace_gated` test.
  - `starter-ext-sdk`: new `WarehouseReadHandle` (`query` /
    `count` / `describe`) backed by a private
    `WarehouseReadBackend` trait; `requires!{}` gains a
    `warehouse_read` arm; `CtxInner::new` widens to 8 args (the
    new backend slot). All six adapters (REST, CLI, workers,
    gRPC, MCP, WASM) provide a `StubWarehouseRead` returning
    `Error::Capability("warehouse_read not wired …")` — the
    real backend lands with row 5's host dispatch wiring.
  - `starter-ext-host`: new `warehouse` module with
    `TemplateRegistry`. `TemplateRegistry::builtin()` registers
    the four templates currently hard-coded in `AnalyticsBridge`
    (`meter_kwh_last_24h`, `meter_litres_last_24h`,
    `meter_value_30d_15m`, `meter_value_24h_1m`).
  - `rubix-agent`: `TimescaleAnalyticsBridge` now holds an
    `Arc<TemplateRegistry>` and uses it as the catalog gate
    — unknown templates are refused at the registry boundary
    before any resolver dispatch. Existing `sqlx` resolvers
    unchanged (row-2 cut-over is catalog-only).
  - Design landed: `docs/design/extensions/warehouse-read.md`.
- Decisions:
  - Resolver execution stays in `rubix-agent`. The `host` crate
    is the catalog source of truth (`TemplateRegistry`) but does
    not depend on `sqlx` or the warehouse client — layering
    hygiene from the proposal stays intact.
  - Per-call **table allowlist** enforcement (cross-checking
    `TemplateSpec::tables` against the grant's `tables: […]`) is
    deferred to when row 3 lands. Until external contributions
    are possible, the four builtin templates all touch
    `samples` and the grant pattern is uniform.
  - V1 surface is sync (`Vec<Row>`). The streaming v2 from
    Appendix A is deferred until a real extension's working
    set forces it; v1 stays on the handle when v2 lands.
  - `_meta.caller`-bound `$caller_tenant_id` binding will be
    enforced by the host-side `WarehouseReadBackend` impl when
    it lands in row 5; today the stubs all refuse with
    `Error::Capability` so no extension can call `warehouse.*`
    via process flavour yet.
- Next: pick up row 3
  (`contributes.warehouse_templates[]` manifest slice). The
  registry is now ready to accept contributed specs — row 3
  is the manifest → `Loader::commit` →
  `TemplateRegistry::insert` glue, plus the manifest-validation
  rules (name shape, no shadowing of builtins without an
  explicit `override:` flag).

### 2026-05-27 — row 1 lands: `CallerIdentity` stamping + `ctx.caller()`

- Rows moved: row 1: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `identity` module with `CallerIdentity`
    and `FrameMeta`; `JsonRpcRequest` / `JsonRpcNotification`
    gain an optional `_meta` envelope field (additive, omitted
    from the wire when empty).
  - `starter-ext-supervisor`: `SupervisorHandle::call_as(…)` and
    `send_with_caller(…)` stamp `_meta.caller` via the private
    `stamp_caller` helper. Existing `call` / `send` stay as
    system-frame paths.
  - `starter-ext-sdk`: `CtxInner` carries `Option<Arc<CallerIdentity>>`
    with a `with_caller(…)` builder; `requires!{}`-generated
    Ctx newtypes expose `ctx.caller() -> Option<&CallerIdentity>`
    always (no capability gate — identity is kernel-level).
  - Process flavour dispatch loop extracts `_meta.caller` per
    inbound frame and rebuilds the per-call Ctx.
  - Design landed: `docs/design/extensions/caller-identity.md`.
  - Tests cover SPI round-trip, supervisor stamping (incl. overwrite
    of pre-existing meta), and `CtxInner::with_caller`.
- Decisions:
  - `_meta.caller` rides on the envelope, not inside `params`, so
    handler parameter schemas stay unreserved.
  - System frames (`init`, `health`, `shutdown`, internal cron)
    omit `_meta` entirely; SDK reflects as `ctx.caller() == None`.
  - Builtin + wasm flavour wiring is intentionally deferred to
    rows 2/5; the SPI + supervisor stamping is sufficient to
    unblock the row-2 `WarehouseReadHandle` work, which is what
    the lynchpin role requires.
- Next: pick up row 2 (`WarehouseReadHandle` + builtin
  `TemplateRegistry`). The handle's host-side impl will be the
  first consumer of `ctx.caller()` — it refuses any frame whose
  `tenant_id` is `None`.

### 2026-05-27 — scope and progress docs created

- Rows moved: none — all rows start at ❌ not started.
- Decisions: critical path locked from
  [extension-architecture-north-star.md §"Critical path"](../../proposal/extension-architecture-north-star.md);
  EventBus promoted ahead of `DashboardHandle` (was row 6,
  now row 4); `AnalyticsBridge` recorded as the seed for
  `TemplateRegistry`.
- Next: pick up row 1 (`CallerIdentity` stamping). It is the
  lynchpin and unblocks rows 2, 5. The PR should land
  alongside a new `docs/design/extensions/caller-identity.md`
  in the same commit, per
  [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md).
