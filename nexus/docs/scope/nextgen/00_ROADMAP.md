# Nexus Next-Gen — Roadmap & Parallel-Session Plan

> Companion to [GAP_ANALYSIS.md](./GAP_ANALYSIS.md). This doc is **how we build the gaps with
> multiple AI sessions running at once** without them colliding. Read this before starting any
> `WS-xx` workstream.

---

## 0. Evidence freshness — re-verify before you build (MANDATORY)

These docs were written against baseline **`82a6a19a` (2026-06-09)** and cite specific files/lines
across nexus *and shared crates* (`starter-*`). **Citations rot** — especially under parallel work and
in shared crates other apps also change. A peer review already caught two stale "already exists"
claims (login-hang was in the wrong crate; `Accept-Units` was described as unbuilt when it ships in
`starter-server`). So, as the **first step of every workstream**:

1. **Re-grep the evidence** your WS depends on (every file:line in its "Current state" / "What
   exists" section). Confirm it still says what the doc claims.
2. If a claim has drifted, **fix the WS doc first** (and update its `Verified:` line), then proceed.
   A wrong "it's just wiring" claim silently inflates or deflates the whole estimate.
3. Each WS carries a **`> Verified: <commit> on <date>`** line under its status header. Bump it when
   you re-verify. Treat anything older than the current branch tip as *unverified*.

This is cheap insurance against the single most likely source of wasted effort: building against a
claim that was true last week.

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
        ┌──── WAVE 0 (do FIRST — a design wave, NOT ½ a day; needs WS-10+WS-11 input) ────┐
        │  C1 Dashboard JSON model schema   (extends ui/src/data/types.ts + DTOs)      │
        │  C2 Macro/param-binder engine contract — VERSIONED (v1=time+vars; params/    │
        │     host_tokens/units reserved). Co-designed w/ WS-10 (§4.2) + WS-11 (§3).   │
        │  C3 URL-state + cache-key tuple (time, vars, units/locale/tz, kind+params)   │
        │  C4 nexus-spi/openapi registration conventions for new areas                 │
        │  C5 Kinds manifest + PanelQuery union (WS-10)                                │
        │  C6 Changelog recording convention + tenant-scoped table (WS-12)             │
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
- **Wave 0 is a *design* wave, not a ½-day chore.** It must settle the contracts that 5+ workstreams
  build against (C1–C6). Critically, **C2 (the binder) can't be fully frozen without WS-10's param
  model and WS-11's cache-key needs** — so Wave 0 pulls in design input from WS-10 and WS-11 (and
  WS-12 for C6), and is **scoped/budgeted as real design work** (think days, with the right people in
  the room), not a single quick session. Under-budgeting Wave 0 is the #1 way the parallel plan
  generates rework — see the C2 versioning note (§6 C2) and the cache-key note (§6 C3 / WS-09 §P1).
- **C2 ships *versioned*, not all-at-once.** v1 = time + variables (unblocks WS-01/02 immediately);
  the signature **reserves** `params` + `host_tokens` (WS-10) and a units/locale context (WS-11) so
  adding them later is additive, not a breaking re-freeze. WS-10/WS-11 design those fields *during*
  Wave 0 so the reserved slots are shaped right, but their *implementation* lands when those WS do.
- **WS-03** ships the **binder** that **WS-01 and WS-02 both consume** — start WS-03 a beat
  before / alongside them. WS-01 ↔ WS-02 are tightly related (both inject into queries) but split
  by file; coordinate on the binder call site only.
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
| **0** | C1–C6 shared contracts (1 session) **+** WS-09 login-hang fix | unblock everyone; fix a live bug |
| **1** | **WS-03 + WS-10 query-kinds** (one effort: binder+macros+kinds+schema+history) · **WS-04** (panel editor) · **WS-07** (alerting) · **WS-09** (cache/rate-limit/quotas) · **WS-11** (prefs-on-PG + Accept-Units convert path) · **WS-12** (changelog-on-PG + record-on-mutate + `GET /audit`) | WS-03/10 unblock W2; the rest are independent high-value |
| **2** | **WS-01** (time range) · **WS-02** (variables) · **WS-06** (flows builder) · **WS-08 + WS-10 datasource-kinds** (connectors-as-declaration) · **WS-11** (series quantity-tagging + UI) · **WS-12** (per-kind `Reversible` + undo/redo + audit UI) | consume the WS-03/10 binder; WS-08 feeds WS-06; WS-11 tags series for conversion |
| **3** | **WS-05** (folders/rows/repeat/JSON/versioning) · WS-07 phase-2 (routing) · WS-09 HA/OTel · WS-10 tenant-authored kinds · WS-12 retention/forget + AI-actor undo | repeat needs WS-02; structure needs C1; WS-12 versioning-overlap settled w/ WS-05 |

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
| **WS-11** | `nexus-api/src/prefs/**` (PG store + mount, new), `routes/me/**`🔶 (resolved prefs), `routes/query/run.rs`🔶 (envelope+convert at edge), `alerting/notify/**`🔶 (render in recipient prefs) | `features/widgets/**`🔶 (label from returned unit/symbol), `datetime/**`🔶 (point useDateTime at resolved prefs), prefs screen (new) | `data/types.ts`🔶 (SeriesField.quantity/storedUnit), `useWidgetQuery.ts`🔶 (Accept-Units), starter-prefs/starter-spi (upstream) |
| **WS-12** | `nexus-api/src/changelog/**` (recorder/undo wiring, new), `nexus-api/src/reversible/**` (per-kind impls + registry, new), `routes/audit/**` (new), every `routes/<kind>/{create,update,delete}.rs`🔶 (drop in `record_if_reversible`) | `features/history/**` (audit + per-resource History, new), undo/redo shortcuts+toasts (small, app-shell) | `routes/mod.rs`🔶, `main.rs`🔶 (mount undo_router + recorder), `data/types.ts`🔶 (Change/audit DTO), starter-changelog/starter-undo (upstream) |

The genuinely hot shared files are: `ui/src/data/types.ts`, `nexus-api/src/routes/mod.rs`,
`nexus-spi/src/openapi.rs`, `nexus-engine/src/registry/{inputs,outputs}.rs`, `main.rs`, the
**WS-03/WS-10 shared binder** (one component, one owner — do not let two sessions write it), and the
**`QueryRequest` DTO + `/query` handler** (five feeders, one owner = WS-03 — see §6 C7). Wave 0
should pre-add the *registration slots/sections* in each so later PRs only append a line.

**WS-12 cross-cutting caveat:** WS-12 needs a one-line `record_if_reversible(...)` call inside each
kind's mutation handlers (`routes/dashboards/*`, `routes/datasources/*`, …) — files **owned by other
workstreams** (WS-05, WS-08, …). To avoid collisions, treat this as a **C6 convention** (Wave 0): the
recorder + registry are mounted in Wave 0, and **each kind's owning workstream adds its own
`record_if_reversible` call + `Reversible` impl** as part of that workstream's PR, following the C6
pattern. WS-12 owns the substrate, the `reversible/` registry, the audit route, and the UI — not edits
to every other team's handlers.

---

## 5. Migration numbering — **per-WS ranges, not a shared sequence** (review fix)

Current latest: `nexus-store/migrations/nexus/0005_tags.sql`. A shared `0006, 0007, …` sequence is
fragile under parallelism — reserved-but-unused numbers + "take the next free" collide the moment two
sessions both need an ad-hoc migration. **Instead, each WS owns a numeric block** and numbers freely
*within* it. No two sessions ever contend for a number, and unused numbers don't strand the sequence.

| Block | Workstream | First/likely migrations |
|---|---|---|
| `06xx` | WS-05 dashboards/structure | `0601_dashboard_model.sql`, `0602_folders.sql`, (`0603_version_tags.sql` — thin tag index per D1, *not* a JSON-snapshot table) |
| `07xx` | WS-02 variables | `0701_variables.sql` (or in-dashboard JSON; table optional) |
| `08xx` | WS-03 query authoring | `0801_query_history.sql` |
| `10xx` | WS-07 alerting | `1001_alert_channels_v2.sql`, `1002_alert_routing.sql` |
| `12xx` | WS-09 prod hardening | `1201_query_cache_meta.sql` (if persisted), `1202_quotas.sql` |
| `13xx` | WS-08 datasources | `1301_datasource_kinds.sql` |
| `14xx` | WS-10 kinds | `1401_query_kinds.sql` (tenant-authored; manifest-only needs none) |
| `15xx` | WS-11 units/prefs | `1501_prefs.sql` (port `starter-prefs/migrations/postgres/0001_*` under RLS) |
| `16xx` | WS-12 audit/undo | `1601_changelog.sql` (port `starter-changelog-postgres/*` + `tenant_id`+RLS), `1602_undo_cursors.sql` (port `starter-undo/*`) |

Pick the **next free number within your block**; record it in your WS doc. WS-01/04/06 need no schema
(state lives in the dashboard JSON model / engine registry). If a WS exhausts its block (unlikely),
take the next free `xxNN` and note it here. *(This supersedes the old flat `0006–0017` list; the old
`0011_audit_log.sql` is gone entirely — audit uses the WS-12 changelog, not a bespoke table.)*

---

## 6. Shared contracts (Wave 0 deliverables) — define these once

### C1 — Dashboard JSON model
The serialised dashboard shape that WS-01/02/04/05 all read/write. Extend
`ui/src/data/types.ts` `Dashboard` with: `timeDefaults?: {from,to,refresh}`,
`variables?: Variable[]`, `panels[].fieldConfig` (overrides), `rows?`, `schemaVersion`. Mirror
in `nexus-spi` DTOs. **This is the contract WS-05's import/export validates against.** Owner:
Wave-0 session. → detailed in [WS-05](./WS-05_DASHBOARD_STRUCTURE.md) §"JSON model".

### C2 — Macro / param-binder engine — **VERSIONED, returns BOUND query (not a SQL string)**
**One engine, two front doors** (raw-SQL panels and kinds) — do NOT let WS-10 spawn a second binder,
and do NOT build a string-substitution engine. **The signature returns a bound query, never a finished
SQL string** — this is the project's injection + tenant-isolation boundary and must enforce it by
construction:
```
fn bind(sql: &str, ctx: &BindCtx) -> Result<BoundQuery, BindError>
struct BoundQuery { sql: String /* with $N placeholders */, args: Vec<SqlValue>, validated_identifiers: Vec<String> }
struct BindCtx {
    // --- v1 (ships first; unblocks WS-01/02) ---
    time_range: Option<TimeRange>, interval: Option<Duration>, variables: BTreeMap<String, VarValue>,
    // --- RESERVED, designed in Wave 0 w/ WS-10, populated when WS-10 lands ---
    params: BTreeMap<String, ParamValue>,  // kind named params (schema-validated upstream)
    host_tokens: HostTokens,               // host-bound: caller_tenant_id, caller_user_id
    // --- RESERVED for WS-11 (units affect rendering, not SQL — likely a separate UnitsCtx at the
    //     convert layer, NOT this binder; Wave 0 confirms it does not belong here) ---
}
```
**Values are ALWAYS bound `$N` args; the only text inserted is a vetted identifier/fragment** (the
`col` in `$__timeFilter(col)`, a `$__timeGroup` bucket literal), recorded in `validated_identifiers`.
**The runner changes too:** `nexus-store/src/query/run.rs:44` is `sqlx::query(sql)` with no arg
channel today — WS-03 updates it to execute `BoundQuery` as a prepared statement. Handles
`$__timeFilter/$__timeGroup/$__interval/$__timeFrom/$__timeTo`, `$var`/`${var:csv}`/`$__sqlIn`; later
kind params + host tokens. **Wave 0 freezes the v1 signature AND the reserved-field shapes** (with
WS-10/WS-11 input) so WS-01/02 build now and WS-10/WS-11 slot in without a re-freeze. Owner: **WS-03**
(co-designed with WS-10/WS-11). → [WS-03](./WS-03_QUERY_AUTHORING.md) §"Macro engine",
[WS-10](./WS-10_KINDS_EXTENSIBILITY.md) §4.2.

### C3 — URL + **cache-key tuple** (the most-coupled artifact — design the full shape in Wave 0)
Time range + variables live in the URL query string (`?from=now-6h&to=now&var-region=Site-A`) for
shareable deep links, and fold into TanStack keys + the WS-09 result-cache key. **The cache key is
fed by five workstreams — design the full tuple once, up front, even before all feeders exist:**
```
key = (tenant, datasource, query_id, time_resolved, vars, units_locale_tz)
        where query_id = interpolated_sql | (kind_name + bound_params)   // WS-03/10
              time_resolved = {from,to} snapped to the refresh tick      // WS-01
              vars          = resolved variable values                    // WS-02
              units_locale_tz = resolved {units, locale, tz}             // WS-11  ← bake in NOW
```
**`units_locale_tz` is a *constant placeholder* until WS-11 populates it** — including the field from
day one means turning WS-11 on later does NOT silently serve cross-unit-poisoned cache entries
(the dangerous ordering the wave plan would otherwise create — see WS-09 §P1). Two-layer scope
(canonical@tenant / converted@user) is decided here too. Owner: **WS-01 + WS-09**, co-designed Wave 0.
→ [WS-01](./WS-01_TIME_RANGE_AND_REFRESH.md) §"State", [WS-09](./WS-09_PRODUCTION_HARDENING.md) §P1.

### C4 — OpenAPI/codegen conventions
New area = new `nexus-spi/src/dto/<area>/` module + a registration line appended to
`openapi.rs`'s paths/components list + new router appended to `routes/mod.rs`. Then
`cargo run --bin openapi > openapi.json && (cd ui && pnpm codegen)`. No hand-written client types.

### C5 — Kinds manifest + `PanelQuery` union shape (WS-10)
The declaration format for a query-kind (`name`, `params_schema`, `sql_file`, `datasource_kind`,
`tables`, optional `cache`) and the **`PanelQuery` discriminated union** (`{mode:"sql",…}` |
`{mode:"kind", kind, params}`) that WS-04 (panel editor "pick a kind" mode) and WS-05 (portable
dashboard JSON) both serialise. Freeze the union shape + manifest field names in Wave 0 so WS-04/05
target a stable `PanelQuery`. Recommend nexus-native field names aligned to the rubix `block.yaml`
mental model (§WS-10 §9 open questions). Owner: **WS-10**. → [WS-10](./WS-10_KINDS_EXTENSIBILITY.md) §4.1.

### C6 — Changelog recording convention (WS-12)
WS-12 mounts the `ChangeRecorder` + `UndoService` + `ReversibleRegistry` in Wave 0, plus the helper
signature each owning workstream calls in its own mutation handlers:
```
record_if_reversible(&registry, &recorder, actor_from(principal), ChangeDraft::{update|create|delete}(resource_ref, before, after)).await?;
```
**The convention (so WS-12 doesn't edit everyone's files):** every workstream that adds a persisted,
mutable resource (WS-02 variables, WS-05 dashboards/folders, WS-08 datasources, WS-10 kinds, WS-07
alert rules, …) **adds the `record_if_reversible` call + a `Reversible` impl for its kind** as part of
*its* PR, following this pattern; WS-12 owns the substrate, registry, audit route, and UI. The
changelog table is **tenant-scoped (`tenant_id` + RLS)** and written inside the `tenant_tx` so RLS
binds. Freeze: the helper signature, the `Reversible` trait usage, and the `tenant_id` requirement.
Owner: **WS-12**. → [WS-12](./WS-12_AUDIT_AND_UNDO.md) §3.2–3.3.

### C7 — `QueryRequest` DTO — single owner for a heavily-overloaded endpoint
`POST /query` accumulates surface from many workstreams: raw `sql` (today) + `{kind, params}` (WS-10)
+ `time_range` (WS-01) + variable values (WS-02) + the `Accept-Units` header (WS-11). That's five
owners on one discriminated-union request — a collision magnet. **`QueryRequest` (and the `/query`
handler) get a single owner: WS-03** (which already owns the binder it feeds). Other workstreams add
their field via a **small, reviewed PR against WS-03's DTO**, not by editing it in parallel. Freeze in
Wave 0: the `PanelQuery`/`QueryRequest` union shape (sql-mode vs kind-mode) + where each WS's field
lives. Treat `QueryRequest` like the binder — one hot contract, one owner. → WS-03, WS-10 §4.1 (C5).

---

## 6a. Wave-0 DECISIONS (committed — not "decide later")

Peer review flagged that several "decide in Wave 0" notes were deferrals of hard calls that two
divergent sessions could each start building against. These are now **decisions**, with an owner. A
Wave-0 session may overturn one *with rationale recorded here*, but the default is committed:

- **D1 — Dashboard history = ONE system (resolves WS-05 ↔ WS-12 overlap).** There is **one ledger**:
  WS-12's changelog. WS-05 "dashboard versions" are **not a second store** — a *version* is a
  **named, user-curated checkpoint that points at a changelog snapshot** (a tagged entry + a label/
  message). No separate `dashboard_versions` JSON copy, no second diff/restore stack: WS-05 reuses
  WS-12's before→after diff + restore. *(Frees migration `0009`.)*
- **D2 — Dashboards use SNAPSHOT, not patch, in their `Reversible` (the constraint behind D1).** WS-12
  §3.3 floated patch for large JSON, but **"restore to version N" needs an absolute state at N**, not
  a patch chain to replay. So dashboards record full `before`/`after` snapshots. If snapshot size
  becomes a real problem, the mitigation is **periodic snapshot + patch-between** (a compaction
  detail), decided then — but the *default and the v1 contract is snapshot*, because versioning
  depends on it. Other kinds still choose per the WS-12 matrix; dashboards are pinned to snapshot.
- **D3 — C2 binder ships versioned** (v1 = time+vars; `params`/`host_tokens`/units reserved) — see §6 C2.
- **D4 — Cache key carries `units_locale_tz` from day one** (constant until WS-11) — see §6 C3 + WS-09 §P1.
- **D5 — `QueryRequest` is a hot shared contract with a single owner** (WS-03), not five — see §6 C7.

Each decision names the workstreams it binds; if you're in one of those WS, treat the decision as
fixed unless a Wave-0 session amends this list.

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

FIRST ACTION (mandatory, ROADMAP §0): re-grep every file:line your WS's "Current state" / "What
exists" section cites and confirm it still holds. If a claim drifted, fix the WS doc + bump its
`Verified:` line BEFORE writing any code. Then confirm the shared contracts you depend on exist, and
propose a short task breakdown.
```

---

## 8. Definition of done (per workstream)

A workstream is done when: its spec's acceptance criteria pass; backend + UI tests are green and
mirrored; the OpenAPI contract + codegen are regenerated and committed; the live integration
suite still passes; the feature is reachable from the UI (not just the API); and its WS doc's
status reads **Done — merged**. Demo-able against the seeded admin on `127.0.0.1:8080`.

**Cross-cutting DoD items every WS must also satisfy:**
- **Re-verified evidence (§0):** the WS's file:line claims were re-grepped and its `Verified:` line bumped.
- **Audit/undo wired (C6 / WS-12):** if the WS adds any persisted mutable kind (dashboard, datasource,
  variable, kind, alert rule, folder, …), it ships that kind's `record_if_reversible` call **and** its
  `Reversible` impl + registry entry, and adds the kind to WS-12's "known mutable kinds" manifest so
  the coverage guard sees it. **A new mutable kind that doesn't record is an incomplete WS**, not a
  WS-12 problem.
- **Cache-key dimensions (C3):** if the WS adds a query-affecting input (time, vars, units, params), it
  is reflected in the cache key (or explicitly marked cache-irrelevant).
