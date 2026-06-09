# Nexus Next-Gen — Dashboarding Gap & Build Plan

This folder is the plan to take Nexus from "a solid query engine with a grid" to a **next-gen,
Grafana/Power-BI-class dashboarding platform**, built by **multiple AI sessions in parallel**.

## Read in this order

1. **[GAP_ANALYSIS.md](./GAP_ANALYSIS.md)** — full review: what exists (a lot — the hard plumbing
   is done), what's missing (the interactive analytics layer), scored against Grafana/Power BI,
   with file:line evidence.
2. **[00_ROADMAP.md](./00_ROADMAP.md)** — how to run many sessions at once without collisions:
   dependency graph, waves, **file-ownership partition**, migration-number reservations, shared
   contracts, and a copy-paste **per-session kickoff prompt**.
3. The workstream you're assigned (**WS-xx**) — your scope, design, and acceptance criteria.

## Workstreams

| # | Workstream | Gap | Wave | Depends on |
|---|---|---|---|---|
| [WS-01](./WS-01_TIME_RANGE_AND_REFRESH.md) | **Time Range & Auto-Refresh** | huge — *the #1 gap* | 2 | WS-03 macros |
| [WS-02](./WS-02_VARIABLES_AND_TEMPLATING.md) | **Variables & Templating** | huge | 2 | WS-03 macros |
| [WS-03](./WS-03_QUERY_AUTHORING.md) | **Query Authoring + Macro Engine** | huge | 1 | — (unblocks 01/02) |
| [WS-04](./WS-04_PANEL_EDITOR.md) | **Panel Editor & Viz Config** | large (cheap — renderers ready) | 1 | C1 model |
| [WS-05](./WS-05_DASHBOARD_STRUCTURE.md) | **Folders/Rows/Repeat/JSON/Versioning** | large | 0+3 | WS-02 (repeat) |
| [WS-06](./WS-06_FLOWS_BUILDER.md) | **Flows Visual Builder** | large — *user's ArkFlow ask* | 2 | WS-08 (palette) |
| [WS-07](./WS-07_ALERTING.md) | **Alerting depth** | medium-large | 1 | — (extend engine) |
| [WS-08](./WS-08_DATASOURCES.md) | **Connector breadth (MQTT/Modbus/…)** | large | 2 | — |
| [WS-09](./WS-09_PRODUCTION_HARDENING.md) | **Cache/Audit/Rate-limit/HA/OTel** | large | 0+1+3 | C1/C3 (cache key) |
| [WS-10](./WS-10_KINDS_EXTENSIBILITY.md) | **"Kinds" — declarative query/datasource extensibility** | strategic — *ports the rubix `kinds/` pattern; reshapes WS-03/08/09* | 0+1+2 | WS-03 binder |
| [WS-11](./WS-11_UNITS_AND_PREFS.md) | **Units & datetime prefs — backend-side conversion** | medium — *mostly wiring existing `starter-prefs`/`starter-spi/units`* | 1+2 | WS-04, WS-10 |
| [WS-12](./WS-12_AUDIT_AND_UNDO.md) | **Audit log + undo/redo — one changelog substrate, for everything** | medium — *mostly wiring existing `starter-changelog`/`starter-undo`* | 1+2 | absorbs WS-09 audit |

## The one-paragraph summary

The foundation is **excellent and verified** — engine seam, multi-tenant RLS, query safety,
connection pooling, a real alert state machine, per-dashboard sharing, secrets, a working
canvas + ECharts + codegen'd client. The gap is the **interactive analytics layer** every BI
power user expects: a **global time range**, **template variables**, **time macros + a real query
editor**, a **deep panel editor**, **dashboard structure** (folders/rows/repeat/JSON/versioning), a
**visual flow builder**, **richer alerting**, **more connectors**, and **production ops** (result
cache, quotas, audit, rate-limit, multi-node, OTel). These are **additive on a sound base**, which
is why they parallelise cleanly.

## The extensibility direction — "Kinds" (WS-10)
A strategic addition ported from the **rubix `kinds/` pattern**
(`rubix/extensions/com.nubeio.rubixos/`): add a queryable API by **dropping files** (a `.sql` + a
`_params.json` schema + an optional `.cache.yaml`), not by recompiling. Named, validated,
parameterized **query-kinds** become the safe default (raw SQL becomes the advanced escape hatch),
tenant isolation is **structural** (a host-bound `$caller_tenant_id` the caller can't supply), and
caching is a **declarative sidecar**. The same idea applied to connectors gives **datasource-kinds**.
This folds three workstreams — **WS-03** (the param binder *is* the macro engine), **WS-08**
(connectors-as-declaration), **WS-09** (the cache sidecar) — into one coherent mechanism, and is what
makes "Ask Nexus" AI panels and GitOps dashboards safe. See [WS-10](./WS-10_KINDS_EXTENSIBILITY.md).

## The localisation direction — backend-side units & datetime (WS-11)
Convert values to each user's **preferred units + date/time format on the *backend*** so every
consumer — the web UI, a future mobile app, alert notifications, exports, the raw API — gets correct
output from one implementation. The platform already has the machinery: `starter-spi/units`
(canonical-SI storage + `uom`-backed convert) + `starter-prefs` (three-layer user→org→default
resolution) + the `SeriesEnvelope` per-series `{quantity, unit}` wire shape. WS-11 is mostly *wiring
that in* (plus finishing the starter `Accept-Units` convert path on Postgres) and *tagging nexus
series with a quantity* so conversion can run. Couples tightly with WS-04 (unit picker → quantity),
WS-10 (a kind declares its output quantities → auto-convert), WS-07 (notify in recipient's units),
and WS-09 (cache key must include resolved units/locale). See [WS-11](./WS-11_UNITS_AND_PREFS.md).

## The history direction — audit + undo are ONE ledger (WS-12)
Audit log and undo/redo are **not two systems** — they're one append-only changelog, and the repo
already designed it: `starter-spi/changelog`'s own docstring says *"five product features collapse
onto this primitive: user audit log, AI-agent log, undo/redo, duplicate, copy/paste."* The
production-grade crates exist (`starter-changelog-postgres` with the `starter_changes` table +
LISTEN/NOTIFY + retention, `starter-undo` with a per-actor redo cursor + `POST /v1/undo|redo`
routes). WS-12 **wires that into nexus** and writes **one `Reversible` impl per resource kind** — so
undo + audit work for **everything** (dashboards, datasources, users, flows, alerts, grants, …) by
following one pattern, not per-feature plumbing. The **audit-log item moves out of WS-09 into WS-12**.
Bonus: AI edits record as `Actor::Agent` (same ledger) and are user-undoable. See
[WS-12](./WS-12_AUDIT_AND_UNDO.md).

## Fastest path to a visible leap (see GAP §5)
1. **WS-09 P0 login-fix** (it's a live bug — hours).
2. **WS-03 macro engine + schema** → **WS-01 time range** (makes every panel time-aware — biggest jump).
   Build the binder as the **WS-10 param engine** from day one (one engine, two front doors).
3. **WS-02 variables** (becomes "real Grafana").
4. **WS-10 query-kinds + core pack** (safe, cacheable, AI/GitOps-friendly query surface) — lands with WS-03.
5. **WS-04 panel editor** (unlocks config the renderers already support; gains a "pick a kind" mode).
6. **WS-05 structure** (folders/JSON/versioning → AI-generated dashboards + GitOps).
7. **WS-09 cache/quotas**, then **WS-06 / WS-07 / WS-08** by business priority.

## Ground rules for every session
- One workstream = one git worktree = one PR. Stay in your **owned files** (ROADMAP §4).
- DTO-first, codegen-driven: `nexus-spi` DTO → `openapi.rs` → `openapi.json` → `pnpm codegen`.
- Use your **reserved migration number** (ROADMAP §5).
- **Extend, don't rebuild** what GAP §3 marks as already-good (engine, RLS, state machine, sharing).
- Ship mirrored tests; keep `cargo test` + `pnpm typecheck/test/build` green; don't break the live
  integration suite.
