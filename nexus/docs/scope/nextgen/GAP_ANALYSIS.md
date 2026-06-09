# Nexus — Next-Gen Dashboarding Gap Analysis

> **Author:** review session 2026-06-09. **Audience:** the humans + the parallel AI sessions
> that will close these gaps. **Baseline:** commit `82a6a19a` on `nexus-backend`.
> **Yardstick:** Grafana 11 + Power BI as the "what a power user expects" bar.
>
> This is the *what's missing and why it matters* doc. The *how we build it, in parallel*
> doc is [00_ROADMAP.md](./00_ROADMAP.md); each gap below points at a workstream spec
> (`WS-xx_*.md`) you can hand to its own AI session.

---

## 0. TL;DR — where we actually are

Nexus is **not** an early prototype. The hard, easy-to-get-wrong plumbing is **done and
verified**:

- ✅ Engine seam (ArkFlow `Stream` → Collector sink → Arrow→JSON; SSE sink → live panels) — real, tested.
- ✅ Multi-tenant **RLS** bound to `Principal.tenant_id`, proven under a non-superuser role with cross-tenant pool-reuse tests.
- ✅ **Query safety** is genuinely enforced server-side: read-only transaction, `statement_timeout`, row/byte caps with truncation, connection-pool cache per `(tenant, datasource)`. Not a stub — see `nexus-store/src/query/run.rs`.
- ✅ **Alerting engine** is real: 10s scheduler with `FOR UPDATE SKIP LOCKED`, a pure `Ok→Pending→Firing→Resolved` state machine, dwell/`for_secs`, silences, transition-only dedup, webhook dispatch.
- ✅ **Per-dashboard sharing** (Grafana-style): view/edit/delete tiers, Manage-gated grants, RLS + grant-check defense in depth.
- ✅ **Tags**, **datasource secrets** (envelope encryption), **flows** (real ArkFlow streams), a **12-col react-grid-layout canvas**, **6+ ECharts panel types**, **codegen'd OpenAPI client**, **74 UI tests + live integration suite**.

**The gap is not the foundation. The gap is the *product surface* that turns "a query engine
with a grid" into "a dashboarding platform a power user picks over Grafana."** Specifically,
the entire **interactive analytics layer** is absent:

| The thing that makes a dashboard a *dashboard* | Status |
|---|---|
| Global **time-range picker** + auto-refresh, fed into every query | ❌ **Absent end-to-end** |
| **Variables / templating** (`$datasource`, `$region`, query-driven dropdowns) | ❌ **Absent** (a vestigial `params` field exists, unused) |
| **Time macros** in SQL (`$__timeFilter`, `$__timeGroup`) | ❌ **Absent** |
| **Schema introspection / autocomplete / query builder** | ❌ Raw `<textarea>` only |
| **Rich panel editor** (thresholds/units/axes/legend/multi-series UI) | ⚠️ Data model has the fields; **no edit UI** |
| **Dashboard structure** (folders, rows, repeat-by-variable, JSON import/export, versioning) | ❌ Flat list, no structure |
| **Flow authoring** (visual node graph, palette, test-run) | ❌ Three raw-JSON textareas |
| **Alerting depth** (multi-condition, no-data policy, email/Slack channels) | ⚠️ Single-scalar + webhook only |
| **Production ops** (result cache, audit log, query history, rate limit, HA, OTel) | ❌ Mostly absent |

So: **the bones are excellent; the muscles a BI power user reaches for daily are missing.**
Closing them is mostly **additive feature work on a sound base**, not rework — which is exactly
why it parallelises well across many sessions.

---

## 1. Scoring against the yardstick

Rough "% of the Grafana/Power-BI expectation a power user would hit." Not precise — a triage
lens for sequencing.

| Capability | Nexus today | Bar | Gap | Workstream |
|---|---:|---:|---:|---|
| Core canvas / grid / panel CRUD | 85% | 100% | small | — (done) |
| Datasource secrets / multi-tenant isolation | 95% | 100% | tiny | — (done) |
| Query safety / governance | 90% | 100% | tiny | WS-09 (quotas) |
| Sharing / per-resource authz | 70% | 100% | medium | WS-05 (public/snapshot) |
| **Time range + refresh** | **5%** | 100% | **huge** | **WS-01** |
| **Variables / templating** | **2%** | 100% | **huge** | **WS-02** |
| **Query authoring (schema/autocomplete/macros)** | **15%** | 100% | **huge** | **WS-03** |
| **Panel editor depth** | **30%** | 100% | **large** | **WS-04** |
| **Dashboard structure (folders/rows/repeat/versioning)** | **10%** | 100% | **large** | **WS-05** |
| **Flow authoring UX** | **10%** | 100% | **large** | **WS-06** |
| Alerting | 40% | 100% | medium-large | WS-07 |
| Datasource breadth (connectors) | 15% | 100% | large | WS-08 |
| Production ops (cache/audit/HA/OTel) | 25% | 100% | large | WS-09 |
| **Declarative extensibility ("kinds")** | **0%** | n/a* | **strategic** | **WS-10** |
| **Units & datetime (backend-side, per-user)** | **10%** | 100% | **medium** | **WS-11** |
| **Audit log + undo/redo (one ledger, all kinds)** | **5%** | 100% | **medium** | **WS-12** |

\* Not a Grafana parity row — it's an *architecture* bet (the rubix `kinds/` pattern) that reshapes
how WS-03/08/09 are built. See §2.10. **Units (WS-11) scores low on *nexus integration* but the
underlying crates (`starter-spi/units` + `starter-prefs`) are ~done — it's wiring, not green-field.**

The five `huge`/`large` rows that are **pure feature gaps on a working base** (WS-01..05) are
where a power user's "this isn't Grafana" reaction comes from. They are the priority and they
are **independent enough to build in parallel** (see the roadmap's dependency graph).

---

## 2. The gaps in detail

### 2.1 Time range & refresh — **the #1 gap** → [WS-01](./WS-01_TIME_RANGE_AND_REFRESH.md)

**Today:** There is no global time picker, no auto-refresh, and no time bounds reach a query.
The UI store (`ui/src/store/ui.ts`) holds only `editMode` + `selectedWidgetId`. A panel query
is `{ datasourceId, sql }` with no `from`/`to` (`ui/src/data/types.ts:34-39`,
`ui/src/features/widgets/useWidgetQuery.ts`). A grep for `timeRange|refresh|now-|$__time`
across the whole UI returns **nothing**.

**Why it's the top gap:** *Every* observability dashboard is "show me X over the last Y." Without
a shared time range, panels can only hardcode windows in their SQL — there is no "last 6h →
last 7d" interaction, no zoom, no shared cross-filter, no live tail. This single feature is the
difference between "a saved query grid" and "a dashboard."

**What "done" looks like:** a Grafana-style picker (quick ranges + absolute + relative `now-6h`),
a refresh-interval selector with auto-refresh, the `{from, to, interval}` carried in URL state
and TanStack Query keys, and `$__timeFilter`/`$__timeGroup` macro substitution so existing SQL
panels become time-aware (couples with WS-03). Zoom-by-drag on time-series panels writes the
range back.

---

### 2.2 Variables & templating → [WS-02](./WS-02_VARIABLES_AND_TEMPLATING.md)

**Today:** No dashboard variables of any kind. The `PanelQuery.params` positional array
(`types.ts:37-39`) is defined but never populated by any UI and never surfaced. No `$var`
interpolation, no query-driven dropdowns, no multi-select, no cascading.

**Why it matters:** Templating is *the* Grafana feature that turns one dashboard into a thousand
("pick a building → all panels re-scope"). Without it you build (and maintain) one dashboard per
entity. For an energy/water/HVAC fleet (the project vision), this is non-negotiable — you cannot
hand-author a dashboard per site.

**What "done" looks like:** dashboard-scoped variable definitions (constant, custom list,
query-driven, datasource-typed, interval, and a built-in `$__dashboard`/`$__user`), a variable
bar UI, multi-select + "All", interpolation into SQL (with `$__sqlIn` style safe expansion),
cascading (one var's query references another), and re-running dependent panels on change.
Persisted in the dashboard model; reflected in URL for shareable deep links.

---

### 2.3 Query / SQL authoring → [WS-03](./WS-03_QUERY_AUTHORING.md)

**Today:** A plain `<textarea>` (`PanelProperties.tsx`, `AddWidgetDialog.tsx`, `Explore.tsx`).
No schema browser, no autocomplete, no validation before run, no query history, no visual
builder. `POST /query` accepts raw SQL only — no server-side templating or time macros
(`nexus-spi/src/dto/query/run.rs`).

**Why it matters:** A power user authoring against an unknown schema with no column hints is
painful, and a *non-SQL* user is locked out entirely. This is also the natural home for the
**time-macro** and **variable** substitution that WS-01/WS-02 depend on.

**What "done" looks like:** (a) a **schema introspection endpoint** (`GET
/datasources/:id/schema` → tables/columns/types) + a browser panel; (b) **CodeMirror SQL** with
schema-aware autocomplete and macro/variable highlighting; (c) **server-side macro engine**
(`$__timeFilter(col)`, `$__timeGroup(col, interval)`, `$__interval`, variable expansion) — the
keystone shared with WS-01/02; (d) **query history** per user; (e) a phase-2 **visual query
builder** for the non-SQL path.

---

### 2.4 Panel editor depth → [WS-04](./WS-04_PANEL_EDITOR.md)

**Today:** You can switch viz type and edit SQL + pick x/value columns. But thresholds,
min/max, decimals, units, per-series label/color, axis config, and legend all **exist in the
data model and are read by the renderers** (`gaugeOption.ts`, `lineOption.ts`) yet **have no
edit UI** (`PanelProperties.tsx` only edits the first series' value column). 10 viz types are
catalogued; the editor exposes a fraction of their config.

**Why it matters:** "I can't set a unit or a threshold or a second series from the UI" is an
immediate credibility hit. The capability is *already in the renderers* — this is mostly a
forms-and-state gap, high value for the effort.

**What "done" looks like:** a tabbed panel editor (Query / Transform / Field & Overrides /
Options) à la Grafana: full thresholds editor, unit picker, decimals, min/max, per-series
overrides (name/color/axis/unit), legend + axis options, value mappings, and a **live preview**
that re-renders as you edit. Plus **field transforms** (rename, calc, group-by, join) as a
client-side pipeline.

---

### 2.5 Dashboard structure → [WS-05](./WS-05_DASHBOARD_STRUCTURE.md)

**Today:** A flat sidebar list. No folders, no rows/sections, no repeat-by-variable, no
JSON import/export, no versioning/snapshots, no duplicate. `starred` exists in the type but
isn't wired. Sharing is solid (view/edit/delete grants) but has **no public/anonymous link,
no snapshot, no embed**.

**Why it matters:** At fleet scale you need organisation (folders), density (rows you can
collapse), the *killer* combo of **repeat-a-row-per-variable-value**, and **dashboard-as-code**
(JSON import/export + versioning) for GitOps and AI-generated dashboards (the "Ask Nexus"
vision in `data/types.ts`).

**What "done" looks like:** folders + move/duplicate; collapsible rows; repeat panel/row by
variable; **dashboard JSON model** import/export (stable schema — already half-promised by the
"stack-agnostic data model" comment); version history with diff + restore; public/snapshot share
links + embed; star/favorites wired.

---

### 2.6 Flow authoring (the "ArkFlow admin tooling" ask) → [WS-06](./WS-06_FLOWS_BUILDER.md)

**Today:** Flows are **real** (FlowManager runs ArkFlow `Stream`s — `nexus-engine/src/flow/manager.rs`),
but authoring is **three raw-JSON `<textarea>`s** (input/pipeline/output) with only JSON-syntax
validation (`flows/FlowFormDialog.tsx`, `flowDraft.ts`). No node palette, no graph, no schema
for node config, no dry-run/test, no live preview of output, no running metrics.

**Why it matters:** This is exactly the user's "more tools to help the admin set this up, test
it, get it working" ask. Today an admin must hand-write ArkFlow config blind. Only `http_poll`
+ `simulator` inputs and `collector`/`sse`/`postgres` outputs are even registered
(`registry/inputs.rs`, `registry/outputs.rs`) — the palette is also tiny.

**What "done" looks like:** (a) a **node-type registry endpoint** exposing each registered
input/processor/output with its config JSON-schema; (b) a **visual node-graph editor**
(React Flow) — drag from palette, connect, edit config via schema-driven forms; (c) **validate +
dry-run** (build the stream, run against a sample/bounded window, show output rows) without
persisting; (d) **live flow metrics** (throughput/lag/last-error) on the flow list; (e) register
the real connectors (couples with WS-08).

---

### 2.7 Alerting depth → [WS-07](./WS-07_ALERTING.md)

**Today:** Engine is solid but **deliberately minimal**: single scalar vs one threshold, 6
operators, webhook-only channels, no-data = non-breaching with no override. The design doc
(`ALERTING.md`) explicitly defers multi-condition, more channels, retry queues, and no-data
policy as *additive* (the enums/traits are built to extend).

**Why it matters:** "alerting is pretty basic" — correct. A power user wants multi-condition
rules, per-series alerting, email/Slack/PagerDuty, notification templating, and a no-data policy.

**What "done" looks like:** multi-condition rules (AND/OR over multiple queries), per-series
evaluation, no-data/error policy toggle, **email + Slack channels** (+ keep webhook), notification
message templating, an **alert list/timeline UI** with state history, "create alert from panel,"
and (stretch) notification policies/routing + grouping. Retry/backoff queue for delivery.

---

### 2.8 Datasource breadth → [WS-08](./WS-08_DATASOURCES.md)

**Today:** **Postgres is the only queryable datasource.** `DatasourceKind` has one variant
(`datasource/shared.rs`). ArkFlow *upstream* speaks Kafka/MQTT/Modbus/HTTP/files, but **none are
registered** — only `http_poll` + `simulator` inputs exist. The "any data source" promise is
currently "Postgres + a simulator."

**Why it matters:** The energy/water/HVAC vision needs **MQTT/Modbus** (live device data) and
likely **Prometheus/InfluxDB/Timescale-native** time-series. The mechanism (ArkFlow registry +
`DatasourceKind`) is there; the connectors aren't wired.

**What "done" looks like:** register + productise MQTT, Modbus, Kafka, and an HTTP/REST query
datasource; add a Prometheus/PromQL-proxy (scoped — it's product-sized per NEXUS.md §3); each
needs a config form, a `test` path, a secret model, and time-series-aware querying. Phased; pick
the 2-3 the business needs first.

---

### 2.9 Production hardening → [WS-09](./WS-09_PRODUCTION_HARDENING.md)

**Today:** `/health` + `/metrics` (Prometheus) + structured `tracing` logs exist. **Missing:**
query-result cache (every panel hits the DB live — a 20-panel dashboard on 10s refresh = 120
QPS/user), query history, **rate limiting** (none), **OpenTelemetry** tracing, per-tenant
**quotas/concurrency caps**, and **multi-node HA** (flow manager + alert scheduler are single-node by
design; in-process SSE broadcast can't span nodes). There's also a known **login-hang bug**: argon2
verify runs on the async runtime without `spawn_blocking` (`TODO-FOR.UI.md`). *(The **audit log** was
listed here originally — it has moved to its own workstream **WS-12**, because audit and undo/redo
share one substrate; see §2.12.)*

**Why it matters:** "scale and into production asap." Without a result cache the DB melts under
refresh load; without rate limits one tenant can starve others; without multi-node you can't
scale out live panels at all.

**What "done" looks like:** result cache (in-proc LRU + optional Redis, keyed by
`tenant+datasource+sql+timerange+vars`), per-tenant query quotas/concurrency, audit log table +
API, query history, rate limiting, OTel traces, the SSE shared-bus (NATS/Redis) for multi-node
live, alert-scheduler leader story, and the argon2 `spawn_blocking` fix. **Fix the login bug
first — it's a live correctness issue.**

---

### 2.10 The extensibility direction — "Kinds" → [WS-10](./WS-10_KINDS_EXTENSIBILITY.md)

**Not a missing feature so much as a missing *shape*** — and the one architectural idea that makes
the others compound. Ported from the **rubix `kinds/` convention**
(`rubix/extensions/com.nubeio.rubixos/` — manifest `block.yaml`, templates `kinds/*.sql` +
`kinds/*_params.json`, cache sidecars `kinds/*.cache.yaml`).

**Today:** you add a queryable thing to nexus by pasting raw SQL into a panel (unsafe, unshareable,
uncacheable) or by writing Rust (a new connector). There is no middle path. Rubix's answer is a
**kind**: a named, JSON-Schema-validated, parameterized query declared as *files* —
tenant isolation is **structural** (a host-bound `$caller_tenant_id` the caller cannot supply),
caching is a **declarative sidecar**, and the `tables:` list drives both capability grants and cache
invalidation. Adding API surface becomes a **file-drop**, not a recompile.

**Why it matters here:** it is *strictly better* than the raw-SQL default and it **folds three of the
gaps above into one mechanism** rather than three bespoke builds:
- **§2.3 (WS-03):** named **query-kinds** become the safe default; the macro engine *is* the kind
  param-binder (one engine, two front doors). Raw SQL drops to the advanced escape hatch.
- **§2.8 (WS-08):** **datasource-kinds** = connectors declared as `{config schema, secrets, test
  query, dialect}` — a file-drop + thin builder instead of an enum edited across DTOs/forms/registry.
- **§2.9 (WS-09):** adopt rubix's **`.cache.yaml`** sidecar wholesale (ttl/scope/invalidate-on-tables
  + time-series bucketing) as the cache spec — a designed format, not a blank page.

It also de-risks two product bets: **AI-generated panels** ("Ask Nexus" picks a *kind + params*,
never arbitrary SQL → bounded, safe, cacheable) and **GitOps dashboards** (kinds are files in a repo).
The one real design delta vs. rubix: nexus is **multi-datasource**, so a query-kind must declare what
it runs against (`datasource_kind` / optional pinned binding) and the `tables:` capability resolves
against the *bound* datasource — with a **mandatory `$caller_tenant_id`-predicate lint** since the
data-side DBs have no RLS (NEXUS.md §5.2). Full design, scope, and open questions in WS-10.

---

### 2.11 Units & datetime — convert backend-side, per user prefs → [WS-11](./WS-11_UNITS_AND_PREFS.md)

**Today:** nexus returns **bare numbers and UTC timestamps**; any unit/format choice would have to
live in each client. There is no quantity model, no per-user unit preference, no server-side
conversion. For an energy/water/HVAC fleet with mixed metric/imperial operators, that means every
client re-implements conversion — and a future mobile app, alert text, and CSV export each get it
wrong differently.

**The good news — the platform already solved this; nexus just hasn't wired it in.** Two existing
crates implement exactly the "convert on the backend" model the user wants:
- **`starter-spi/units/`** — canonical-SI storage + `uom`-backed `normalize_for_storage` /
  `from_canonical` / `convert_for_display`, closed `Quantity` + `Unit` enums, `GET /v1/units`.
- **`starter-prefs/`** — three-layer (user → org → default) preference resolution with `"auto"`
  derivation, `ResolvedPreferences` (timezone, units per quantity, date/time/number formats, locale,
  currency), a prefs middleware, and the **`SeriesEnvelope`** wire shape (`{quantity, unit}` once per
  series). The architecture matches the project memory note on i18n/unit prefs.

**Why backend-side is the right call (the user's instinct is correct):** one implementation serves
the web UI, a **future native app**, **alert notifications** (render in the recipient's units), and
**exports** — instead of N client re-implementations. It's the platform's own mandated rule ("convert
at the presentation edge, never in storage").

**The gap is integration, not invention:** (a) finish the starter **`Accept-Units` / `UnitsCtx`**
convert path (it's a deferred "Phase 2" scaffold); (b) stand up **`starter-prefs` on Postgres** (it's
sqlite-only today, nexus is PG+RLS); (c) **tag nexus series with a `quantity`** (panels/kinds declare
what a column *is*) so conversion can run, with bare-number passthrough for untagged series. Couples
with WS-04 (unit picker → quantity), WS-10 (a kind declares output quantities → auto-convert), WS-07
(notify in recipient units), WS-09 (cache key must include resolved units/locale). Full plan in WS-11.

> 🚧 **Known cross-repo blocker:** `starter-spi`'s `Quantity` enum is **closed** and lacks core
> energy/water/HVAC quantities — most glaringly **volumetric flow-rate** (plus likely mass-flow,
> current/voltage, humidity, ppm). Adding them is an **upstream `starter-spi` change** on the
> critical path, not a nexus-local edit — raise it in Wave 0 (WS-11 header). Until then those
> measures pass through as bare numbers (no conversion).

---

### 2.12 Audit log + undo/redo — one ledger, for everything → [WS-12](./WS-12_AUDIT_AND_UNDO.md)

**Today:** no audit log a user can query (datasource decrypts are *logged* to stdout, nothing is
persisted/queryable) and **no undo/redo anywhere** — a fat-fingered dashboard edit or a deleted
datasource is gone. For a multi-tenant production system this is a real gap: compliance needs "who
changed what," and operators need to recover from mistakes (and from **AI-made** changes once "Ask
Nexus" lands).

**The better-long-term answer is already designed and built in this repo — and it's *one* system,
not two.** `starter-spi/changelog`'s docstring states it outright: *"five product features collapse
onto this primitive: user audit log, AI-agent log, undo/redo, duplicate, copy/paste."* The
production-grade crates exist:
- **`starter-spi/changelog`** — the `Change` envelope (`actor, resource, op, before, after, group_id,
  correlation`), the `ChangeRecorder` write path, and `Reversible` (the one per-kind extension point).
- **`starter-changelog-postgres`** (+ sqlite) — the **`starter_changes`** append-only table, a paged
  `ChangeLog` query trait (`PgChangeLog`), LISTEN/NOTIFY tailing, retention/prune, per-kind policy,
  and GDPR `forget()` tombstoning.
- **`starter-undo`** — per-actor undo/redo over the ledger (`UndoService`, a CAS-on-epoch redo
  cursor, `record_if_reversible` to drop into handlers, `POST /v1/undo|redo` routes).

**Why one ledger is "better long term" (the user's framing):** audit = *read* the ledger; undo/redo =
*replay* the ledger; duplicate/paste = `clone_with` on the ledger. Adding a new audited+undoable kind
(users, dashboards, datasources, flows, alerts, grants, …) = **register one `Reversible` + emit one
`ChangeDraft`** after the mutation — no per-feature audit plumbing, no separate undo stacks.

**The gap is integration, not invention:** (a) mount `starter_changes` + the undo cursor on nexus'
**Postgres+RLS** (add `tenant_id` + RLS, write inside the tenant tx); (b) call `record_if_reversible`
in **every nexus mutation handler**; (c) write **one `Reversible` per kind** (snapshot vs patch per
the crate's matrix); (d) add the missing **`GET /api/v1/audit` endpoint** (the only sizeable net-new
piece — `PgChangeLog` already queries) + undo/redo UI. **This absorbs the "audit log" item from §2.9
(WS-09).** Bonus: AI edits record as `Actor::Agent{run_id,model}` → the AI-agent log is the same
ledger and AI changes are user-undoable. Full plan in WS-12; coordinate the history-overlap with
WS-05 dashboard versioning (§see WS-12 §7).

---

## 3. What's *already good* (don't rebuild it)

To keep the parallel sessions from "improving" solid code:

- **Engine seam, query safety, RLS, connection pooling** — done, tested. Build *on* them.
- **Alert state machine** (`alerting/transition.rs`) — pure, unit-tested; WS-07 *extends*, never rewrites it.
- **Sharing/authz** (`authz/`, `authz/dashboard_instances.rs`) — WS-05 adds public/snapshot *next to* it.
- **The stack-agnostic data model** (`ui/src/data/types.ts`) — the right seam; WS-01/02/04/05 *extend* this type, they don't fork it.
- **OpenAPI codegen** — every new endpoint must flow through `nexus-spi` DTOs → `openapi.json` → `pnpm codegen`. Don't hand-write client types.

---

## 4. Cross-cutting decisions to lock before parallel work

These touch multiple workstreams; settle them up front (see roadmap §"Shared contracts"):

1. **The dashboard JSON model** (WS-05) is the schema WS-01 (time defaults), WS-02 (variables),
   and WS-04 (panel options) all serialise into. **Define it first** so the others target a
   stable shape. → roadmap "Wave 0".
2. **The server-side macro/interpolation engine** (WS-03) is consumed by WS-01 (`$__timeFilter`)
   and WS-02 (variable expansion). **One engine, one owner.** → WS-03 ships the contract early.
3. **Time-range + variable state lives in the URL** (shareable deep links) and in TanStack Query
   keys (cache correctness). Agree the URL param scheme once.
4. **Everything goes through `nexus-spi` DTOs → OpenAPI → codegen.** No hand-written wire types.

---

## 5. Recommended priority (the "into production asap" path)

If the business wants maximum perceived leap per week:

1. **WS-09 login-hang fix** (hours — it's a bug, do it now).
2. **WS-03 macro engine + schema endpoint** + **WS-01 time range** (together they make every
   existing panel time-aware — the single biggest visible jump).
3. **WS-02 variables** (turns the product into "real Grafana").
4. **WS-04 panel editor** (unlocks the config the renderers already support — cheap, high-visibility).
5. **WS-05 dashboard structure** (folders/JSON/versioning — enables AI-generated dashboards + GitOps).
6. **WS-09 result cache + quotas** (before real refresh load hits prod).
7. **WS-06 flows builder**, **WS-07 alerting**, **WS-08 connectors** — in parallel, business-priority order.

See [00_ROADMAP.md](./00_ROADMAP.md) for the wave/dependency plan and the per-session kickoff
prompts.
