# Nexus Next-Gen — Roadmap & Parallel-Session Plan

> Companion to [GAP_ANALYSIS.md](./GAP_ANALYSIS.md). This doc is **how we build the gaps with
> multiple AI sessions running at once** without them colliding. Read this before starting any
> `WS-xx` workstream.

---

## 1. How to run multiple sessions in parallel (the operating model)

Each `WS-xx_*.md` is a **self-contained workstream** scoped so a separate AI session can own it
end-to-end. To make N sessions safe at once:

1. **One workstream = one git worktree = one session.** Don't run two sessions in the same
   working copy. Use `git worktree add ../nexus-ws01 nexus-backend` (or spawn the session with
   `isolation: worktree`). Each lands its own PR.
2. **File ownership is pre-partitioned** (§4 table) so two sessions rarely touch the same file.
   Where they must (shared files like `routes/mod.rs`, `data/types.ts`, `openapi.rs`), that file
   is a **Wave-0 shared contract** edited once, up front, before parallel work starts — then it's
   append-mostly and merge conflicts stay trivial.
3. **DTO-first, codegen-driven.** Backend changes go `nexus-spi` DTO → register in `openapi.rs`
   → regenerate `openapi.json` → `pnpm codegen`. Two sessions adding DTOs touch *different files*
   under `nexus-spi/src/dto/<area>/`; only the `openapi.rs` registration list is shared (append).
4. **Each session owns its migration file**, numbered to avoid collision (§5).
5. **Each PR must:** keep `cargo test` + `pnpm typecheck && pnpm test && pnpm build` green, ship
   its mirrored tests, update its `WS-xx` doc's "status" line, and not break the live integration
   suite.

---

## 2. Dependency graph (what blocks what)

```
        ┌────────────────── WAVE 0 (do FIRST, single session, fast) ──────────────────┐
        │  C1 Dashboard JSON model schema   (extends ui/src/data/types.ts + DTOs)      │
        │  C2 Macro/param-binder engine contract   (WS-03 ships it; WS-10 kinds reuse it) │
        │  C3 URL-state + TanStack-key scheme for {timeRange, vars}                     │
        │  C4 nexus-spi/openapi registration conventions for new areas                 │
        └──────────────────────────────────┬──────────────────────────────────────────┘
                                            │ unblocks
        ┌───────────────────────────────────┼───────────────────────────────────────────┐
        ▼                ▼                    ▼                  ▼            ▼            ▼
   ┌─────────┐     ┌──────────┐        ┌──────────┐      ┌──────────┐  ┌─────────┐  ┌─────────┐
   │  WS-03  │────▶│  WS-01   │        │  WS-02   │      │  WS-04   │  │  WS-06  │  │  WS-08  │
   │ query/  │     │ time     │        │ variables│      │ panel    │  │ flows   │  │ datasrc │
   │ macros  │     │ range    │◀──────▶│ template │      │ editor   │  │ builder │  │ breadth │
   └─────────┘     └──────────┘        └────┬─────┘      └──────────┘  └────┬────┘  └────┬────┘
        │                                   │                                │           │
        │ (macro engine consumed by WS-01/02)                               │ WS-08 connectors
        ▼                                   ▼                                │ feed WS-06 palette
   ┌──────────┐                       ┌──────────┐                          │           │
   │  WS-05   │◀──────────────────────│ (vars in │                          └─────┬─────┘
   │ dashboard│  (JSON model = C1;    │  JSON    │                                │
   │ structure│   repeat needs vars)  │  model)  │                          ┌─────▼─────┐
   └──────────┘                       └──────────┘                          │   WS-07   │
                                                                            │ alerting  │ (mostly independent;
   ┌──────────┐                                                             └───────────┘  email/slack standalone)
   │  WS-09   │  production hardening — mostly independent; LOGIN-HANG FIX = do immediately
   └──────────┘  (result cache key depends on C1+C3 vars/timerange shape)
```

**Reading it:**
- **Wave 0** is a short, single-session prerequisite. It freezes the shared contracts so the rest
  don't fight. ~½–1 day.
- **WS-03** ships the **macro engine** that **WS-01 and WS-02 both consume** — start WS-03 a beat
  before / alongside them. WS-01 ↔ WS-02 are tightly related (both inject into queries) but split
  by file; coordinate on the macro-engine call site only.
- **WS-04, WS-06, WS-07, WS-08, WS-09** are largely independent and can start immediately after
  Wave 0 (WS-09's login fix needs *nothing* — do it now).
- **WS-05** wants the JSON model (C1) and consumes variables (WS-02) for repeat — start its
  folders/versioning parts early, do repeat-by-variable after WS-02 lands its var model.
- **WS-10 (kinds)** is not a separate node above because it *reshapes* WS-03/08/09 rather than
  sitting beside them: its query-kind param-binder **is** the C2 engine WS-03 ships (build it once,
  two front doors), its datasource-kinds **are** WS-08 expressed declaratively, and its
  `.cache.yaml` sidecar **is** the WS-09 cache spec. Treat WS-10 §4–5 as a constraint on how those
  three are built, decided in Wave 0, not as work bolted on after.
- **WS-10 (kinds)** is **strategically coupled to WS-03**: its query-kind *param binder* and WS-03's
  *macro engine* are the **same component** (C2). Build them as one. WS-10's query-kinds land with
  WS-03; its datasource-kinds land with WS-08 (Wave 2). See [WS-10](./WS-10_KINDS_EXTENSIBILITY.md).

---

## 3. Suggested waves (calendar-agnostic, by dependency)

| Wave | Workstreams (parallel within a wave) | Why here |
|---|---|---|
| **0** | C1–C4 shared contracts (1 session) **+** WS-09 login-hang fix | unblock everyone; fix a live bug |
| **1** | **WS-03 + WS-10 query-kinds** (one effort: binder+macros+kinds+schema+history) · **WS-04** (panel editor) · **WS-07** (alerting) · **WS-09** (cache/audit/rate-limit) | WS-03/10 unblock W2; the rest are independent high-value |
| **2** | **WS-01** (time range) · **WS-02** (variables) · **WS-06** (flows builder) · **WS-08 + WS-10 datasource-kinds** (connectors-as-declaration) | consume the WS-03/10 binder; WS-08 feeds WS-06 |
| **3** | **WS-05** (folders/rows/repeat/JSON/versioning) · WS-07 phase-2 (routing) · WS-09 HA/OTel · WS-10 tenant-authored kinds | repeat needs WS-02; structure needs C1 |

You can compress: WS-03 + WS-01 + WS-02 can run in one wave by three sessions if they agree the
binder call-site contract in Wave 0 (C2). The graph, not the calendar, is the constraint. **WS-10
query-kinds should NOT be split from WS-03** — one team owns the shared binder.

---

## 4. File-ownership partition (collision avoidance)

Primary directories each workstream **owns** (creates/edits freely). Files marked 🔶 are
**shared** — edit only in Wave 0 or via small append-only diffs, coordinate.

| WS | Backend owns | UI owns | Shared (🔶 append-only) |
|---|---|---|---|
| **C0/Wave0** | `nexus-spi/src/dto/dashboard/*` (model), `openapi.rs` | `ui/src/data/types.ts`, `ui/src/api/types.ts` | these files become 🔶 after |
| **WS-01** | `routes/query/*` (timerange passthrough) | `features/time/**` (new), `store/time.ts` (new) | `data/types.ts`🔶, `useWidgetQuery.ts`🔶 |
| **WS-02** | `nexus-spi/src/dto/variable/**` (new), `routes/variables/**` (new) | `features/variables/**` (new), `store/variables.ts` (new) | `data/types.ts`🔶, `routes/mod.rs`🔶 |
| **WS-03** | `nexus-engine/src/macro/**` or `nexus-store/src/query/macro.rs` (new), `routes/datasources/schema.rs` (new), `routes/query/history.rs` (new) | `features/query-editor/**`, CodeMirror dep | `routes/mod.rs`🔶, `openapi.rs`🔶 |
| **WS-04** | — (renderers already read the fields) | `features/canvas/PanelProperties.tsx`, `features/widgets/**` (option-builders), `features/canvas/PanelEditor/**` (new) | `data/types.ts`🔶 (overrides) |
| **WS-05** | `routes/dashboards/**` (folders/versions/export), `nexus-store/src/dashboard/**`, `nexus-store/src/folder/**` (new) | `features/dashboards/**`, `features/folders/**` (new) | `routes/mod.rs`🔶, `data/types.ts`🔶 |
| **WS-06** | `nexus-engine/src/registry/**` (node-schema export), `routes/flows/nodes.rs` (new), `routes/flows/dryrun.rs` (new) | `features/flows/**` (React Flow editor) | `routes/mod.rs`🔶, `registry/{inputs,outputs}.rs`🔶 |
| **WS-07** | `nexus-api/src/alerting/**` (extend), `alerting/notify/{email,slack}.rs` (new), `nexus-store/src/alert/**` | `features/alerts/**` | `data/types.ts`🔶 (n/a mostly) |
| **WS-08** | `nexus-engine/src/source/**` (new connectors), `registry/inputs.rs`🔶, `nexus-store/src/datasource/**` | `features/datasources/**` (per-kind forms) | `datasource/shared.rs`🔶 (KIND enum), `registry/inputs.rs`🔶 |
| **WS-09** | `nexus-api/src/{cache,audit,ratelimit,quota}/**` (new), `middleware/**`, `main.rs`🔶 | minimal (audit/history views) | `main.rs`🔶, `serve.rs`🔶 |
| **WS-10** | `nexus-api/src/kinds/**` (registry/loader, new), `kinds/*` pack (sql+schema+cache files), `routes/query/run.rs`🔶 (kind dispatch) | `features/query-editor/**` (kind picker, shared w/ WS-04) | `data/types.ts`🔶 (PanelQuery union), shares the WS-03 binder (NOT a 2nd engine) |

The genuinely hot shared files are: `ui/src/data/types.ts`, `nexus-api/src/routes/mod.rs`,
`nexus-spi/src/openapi.rs`, `nexus-engine/src/registry/{inputs,outputs}.rs`, `main.rs`, and the
**WS-03/WS-10 shared binder** (one component, one owner — do not let two sessions write it). Wave 0
should pre-add the *registration slots/sections* in each so later PRs only append a line.

---

## 5. Migration numbering (avoid collisions)

Current latest: `nexus-store/migrations/nexus/0005_tags.sql`. Reserve ahead so parallel sessions
don't both grab `0006`:

| Number | Workstream | Tables |
|---|---|---|
| `0006_dashboard_model.sql` | Wave 0 / WS-05 | dashboard JSON columns, folders |
| `0007_variables.sql` | WS-02 | dashboard_variables |
| `0008_query_history.sql` | WS-03 | query_history |
| `0009_dashboard_versions.sql` | WS-05 | dashboard_versions |
| `0010_alert_channels_v2.sql` | WS-07 | channel kind extensions, routing |
| `0011_audit_log.sql` | WS-09 | audit_log |
| `0012_query_cache_meta.sql` | WS-09 | (if cache metadata persisted) |
| `0013_datasource_kinds.sql` | WS-08 | per-kind config columns |
| `0014_query_kinds.sql` | WS-10 | tenant-authored kinds registry (optional; manifest-only kinds need no table) |

If a session needs a migration not listed, it takes the **next free number above 0014** and
updates this table in its PR.

---

## 6. Shared contracts (Wave 0 deliverables) — define these once

### C1 — Dashboard JSON model
The serialised dashboard shape that WS-01/02/04/05 all read/write. Extend
`ui/src/data/types.ts` `Dashboard` with: `timeDefaults?: {from,to,refresh}`,
`variables?: Variable[]`, `panels[].fieldConfig` (overrides), `rows?`, `schemaVersion`. Mirror
in `nexus-spi` DTOs. **This is the contract WS-05's import/export validates against.** Owner:
Wave-0 session. → detailed in [WS-05](./WS-05_DASHBOARD_STRUCTURE.md) §"JSON model".

### C2 — Macro / interpolation engine
A single server-side function `interpolate(sql, ctx) -> sql` where `ctx = {timeRange, interval,
variables}`. Handles `$__timeFilter(col)`, `$__timeGroup(col, '5m')`, `$__interval`,
`$__timeFrom/$__timeTo`, and `$var` / `${var:csv}` / `$__sqlIn(var)` expansion with **safe
quoting** (this is an injection boundary — see WS-03 §security). Owner: **WS-03**, but the
*signature* is frozen in Wave 0 so WS-01/02 can call it. → [WS-03](./WS-03_QUERY_AUTHORING.md) §"Macro engine".

### C3 — URL + cache-key state scheme
Time range and variables live in the URL query string (`?from=now-6h&to=now&var-region=Site-A`)
for shareable deep links, and are folded into TanStack Query keys so cache invalidates correctly.
Agree the param names + key tuple once. → [WS-01](./WS-01_TIME_RANGE_AND_REFRESH.md) §"State".

### C4 — OpenAPI/codegen conventions
New area = new `nexus-spi/src/dto/<area>/` module + a registration line appended to
`openapi.rs`'s paths/components list + new router appended to `routes/mod.rs`. Then
`cargo run --bin openapi > openapi.json && (cd ui && pnpm codegen)`. No hand-written client types.

---

## 7. Per-session kickoff prompt (copy-paste template)

When you start a session for a workstream, seed it with:

```
You are implementing <WS-xx: Title> for the Nexus dashboarding platform.

READ FIRST, IN ORDER:
1. nexus/docs/scope/nextgen/GAP_ANALYSIS.md   (why this matters)
2. nexus/docs/scope/nextgen/00_ROADMAP.md     (parallel rules, your file ownership row in §4,
                                               your migration number in §5, shared contracts §6)
3. nexus/docs/scope/nextgen/<WS-xx>_*.md       (your spec — the source of truth for scope)
4. nexus/docs/scope/NEXUS.md                    (architecture invariants you must not break)

RULES:
- Work in your own git worktree/branch off `nexus-backend`. One PR for this workstream.
- Stay inside your owned files (ROADMAP §4). Touch a 🔶 shared file only as a tiny append; if a
  shared contract is missing, STOP and flag it rather than redefining it.
- DTO-first: nexus-spi DTO → openapi.rs → regenerate openapi.json → pnpm codegen. Never hand-edit
  generated client types.
- Use migration number <N> from ROADMAP §5.
- Ship mirrored tests. Keep `cargo test` and `pnpm typecheck && pnpm test && pnpm build` green.
- Do NOT rebuild things GAP_ANALYSIS §3 marks as already-good. Extend, don't fork.
- Update the "Status" line at the top of your WS doc as you progress.

Start by confirming the shared contracts you depend on exist; then propose a short task
breakdown before writing code.
```

---

## 8. Definition of done (per workstream)

A workstream is done when: its spec's acceptance criteria pass; backend + UI tests are green and
mirrored; the OpenAPI contract + codegen are regenerated and committed; the live integration
suite still passes; the feature is reachable from the UI (not just the API); and its WS doc's
status reads **Done — merged**. Demo-able against the seeded admin on `127.0.0.1:8080`.
