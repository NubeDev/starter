# WS-02 — Variables & Templating

> **Status:** Not started · **Wave:** 2 · **Owner:** _unassigned_
> **Depends on:** C2 macro engine (WS-03), C1 JSON model, C3 URL-state (Wave 0) · pairs with WS-01
> **Migration:** block `07xx` (e.g. `0701_variables.sql`; may be unneeded if vars live in dashboard JSON) · **Read first:** GAP_ANALYSIS §2.2, ROADMAP §0 + §6
> **Verified:** `72ae8e12` on 2026-06-09 — re-grepped this WS's file:line claims.

## Goal
Grafana-class **dashboard variables**: one dashboard, parameterised. A variable bar lets the user
pick values (`$region`, `$datasource`, `$building`); panels re-query against the selection; values
deep-link via URL. This is the feature that lets **one dashboard serve a whole fleet** instead of
hand-authoring one per site — directly serving the energy/water/HVAC vision.

## Current state (evidence)
- **No variables (UI/persistence) yet.** A vestigial `PanelQuery.params` positional array
  exists (`ui/src/data/types.ts:37-38`) but is never populated or surfaced.
- No dropdowns, multi-select, cascading, or variable persistence anywhere.
- **SQL-side interpolation is already shipped by WS-03** (re-verify, drift in our favour):
  the C7 `QueryRequest.variables: Vec<QueryVariable>` field
  (`nexus-spi/src/dto/query/run.rs`) and the binder's `$var`/`${var:csv}`/
  `${var:singlequote}`/`$__sqlIn(var)` expansion (`nexus-store/src/query/bind/vars.rs`,
  lowered by `request.rs`) exist and are injection-safe by construction (every value binds
  as a `$N` arg). So WS-02 supplies the variable *definitions*, *resolution* (incl.
  cascading), the *UI*, and the *wiring of resolved values into the query body* — the
  interpolation engine itself is WS-03's and is not rebuilt here.

## Scope
1. **Variable model** (DTO `nexus-spi/src/dto/variable/**` + UI type in `data/types.ts`, C1):
   a `Variable` is `{ name, label?, type, ...typeConfig, current: value|value[], multi, includeAll }`.
   **Types to support:**
   - `constant` — fixed value (often hidden).
   - `custom` — static comma list of options.
   - `query` — options come from running SQL against a datasource (returns one column → option list).
   - `datasource` — options are the tenant's datasources of a given kind (so `$ds` can drive panels).
   - `interval` — a list of durations (drives `$__interval` overrides).
   - `textbox` — free text.
   - built-ins: `$__dashboard`, `$__user`, `$__from`, `$__to` (read-only, from context).
2. **Persistence**: variables live in the dashboard JSON model (C1); a `0701_variables.sql` table (WS-02 `07xx` block) if
   we store them relationally instead of in the dashboard JSONB — **decide with WS-05** (prefer
   in-model JSON to keep import/export self-contained; the migration may be unused).
3. **Variable bar UI** (`ui/src/features/variables/**`): renders each visible variable as a
   single/multi-select (or textbox); "All" option; mount above the canvas in `DashboardPage.tsx`.
4. **Variable editor** (in the dashboard settings, coordinate with WS-05): add/edit/reorder/delete
   variables, pick type, write the option query, preview resolved options, set multi/includeAll.
5. **Interpolation** via the **WS-03 macro engine** (C2): `$var`, `${var}`, `${var:csv}`,
   `${var:singlequote}`, and `$__sqlIn(var)` → safe `IN ('a','b')` expansion. **This is an injection
   boundary** — quoting/escaping is mandatory and lives in the engine, server-side.
6. **Cascading**: a `query` variable's SQL may reference another variable; changing the parent
   re-resolves the child. Build a dependency order; detect cycles.
7. **Re-query on change**: changing a variable invalidates exactly the panels (and child variables)
   that reference it. Fold variable values into TanStack keys + WS-09 cache key (C3).
8. **URL state (C3)**: `?var-region=Site-A&var-region=Site-B` (repeatable for multi); restore on load.
9. **Repeat-by-variable hook**: expose the resolved value list so **WS-05** can repeat panels/rows.

## Design notes
- **Resolution order**: built-ins → constants/custom → datasource → query (topologically by
  dependency). Resolve on dashboard load and on any parent change; cache resolved option lists per
  `(var, parent-values, timeRange)`.
- **`query` variable safety**: same read-only/timeout/caps guards as panel queries
  (`nexus-store/src/query/run.rs`) — reuse, don't reinvent.
- **Multi + `All`**: `All` can mean "every option" (expand to full `IN`) or a `.*` wildcard token —
  support the explicit-expansion form first (predictable, pushdown-friendly).
- **Keep `PanelQuery.params`** for genuine positional binding; variables are a *separate*,
  higher-level concept that interpolates into SQL text before binding.

## Acceptance criteria
- [ ] **C6 (audit/undo):** the `dashboard_variable` kind has a `Reversible` impl + a
  `record_if_reversible` call in its create/update/delete handlers + is in WS-12's mutable-kinds
  manifest; create/edit/delete a variable produces a `Change` row and is undoable. *(If variables live
  in the dashboard JSON, they're covered by the dashboard kind — confirm with WS-12, don't double-record.)*
- [ ] Create a `query` variable; its dropdown is populated from live SQL.
- [ ] A panel using `region = '$region'` (or `$__sqlIn(region)`) re-queries when the selection changes.
- [ ] Multi-select + "All" produce correct, safely-quoted `IN (...)`.
- [ ] Cascading: changing `$datacenter` re-resolves `$host`; cycles are rejected with a clear error.
- [ ] Variable values deep-link via URL and restore on reload.
- [ ] Injection test: a value containing `'); DROP ...` is safely quoted, never executed.
- [ ] Tests: resolution ordering, cycle detection, interpolation/quoting, multi/All expansion,
  dependency-driven invalidation.

## Out of scope (hand off)
- The macro/interpolation engine internals → **WS-03**.
- Repeat-by-variable *rendering* → **WS-05** (this WS exposes the value list).
- Time range → **WS-01** (built-ins `$__from/$__to` come from there).
