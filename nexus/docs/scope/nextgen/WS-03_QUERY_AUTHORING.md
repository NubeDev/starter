# WS-03 — Query & SQL Authoring (Macro Engine · Schema · Autocomplete · History · Builder)

> **Status:** Not started · **Wave:** 1 (ships the macro engine that WS-01/02 need) · **Owner:** _unassigned_
> **Depends on:** C4 OpenAPI conventions (Wave 0) · **Unblocks:** WS-01, WS-02
> **Migration:** block `08xx` (e.g. `0801_query_history.sql`) · **Read first:** GAP_ANALYSIS §2.3, ROADMAP §0 + §6 (C2)
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims before building (ROADMAP §0).
>
> ⚠️ **Coordinate tightly with [WS-10 (Kinds)](./WS-10_KINDS_EXTENSIBILITY.md).** The macro engine
> below (C2) and WS-10's **query-kind param-binder are the SAME component** — one engine, two front
> doors (macros in raw SQL *and* named-param binding for kinds, plus host-bound tokens like
> `$caller_tenant_id`). **One session owns it; do not build two binders.** The C2 signature must
> carry the param map + host-token set WS-10 needs — freeze it in Wave 0. Land WS-03 query authoring
> and WS-10 query-kinds together.

## Goal
Make authoring queries pleasant and powerful, and **ship the server-side macro/interpolation
engine (C2)** that WS-01 (time) and WS-02 (variables) both depend on. Replace the raw `<textarea>`
with a schema-aware editor, expose datasource schema, add query history, and (phase 2) a visual
query builder for non-SQL users.

## Current state (evidence)
- Plain `<textarea>` everywhere: `PanelProperties.tsx`, `AddWidgetDialog.tsx`,
  `query-editor/Explore.tsx`. No autocomplete, schema, validation, or history.
- `POST /query` = raw SQL, no templating/macros: `nexus-spi/src/dto/query/run.rs`,
  `nexus-store/src/query/run.rs` (guards are good — reuse them).
- Result schema *is* returned (`ColumnSchema`) and shown in `Explore` results, but there's no
  *pre-query* schema introspection.

## Scope
### A. Macro / interpolation engine (C2 — the keystone, do first)
A single server-side entry point, e.g. `nexus-store/src/query/macro.rs` or
`nexus-engine/src/macro/`:
```
fn interpolate(sql: &str, ctx: &MacroCtx) -> Result<String, MacroError>
struct MacroCtx {
    time_range: Option<TimeRange>,
    interval:   Option<Duration>,
    variables:  BTreeMap<String, VarValue>,   // WS-02 dashboard variables
    params:     BTreeMap<String, ParamValue>, // WS-10 kind named params (schema-validated upstream)
    host_tokens: HostTokens,                  // WS-10 host-bound: caller_tenant_id, caller_user_id
}
```
This is the **single binder shared with [WS-10](./WS-10_KINDS_EXTENSIBILITY.md)** (C2). It serves
*both* raw-SQL macros and kind named-param binding — **do not build a second engine.**

Supported macros (Grafana-aligned):
- `$__timeFilter(col)` → `col >= '<from>' AND col < '<to>'`
- `$__timeGroup(col, '5m')` / `$__timeGroup(col, $__interval)` → dialect time bucket
- `$__timeFrom` / `$__timeTo` → literal timestamps
- `$__interval` → auto bucket (from WS-01's max_data_points) or the chosen interval var
- `$var`, `${var}`, `${var:csv}`, `${var:singlequote}`, `$__sqlIn(var)` → variable expansion
- `$<param>` → WS-10 kind named param (bound positionally where possible, else safely quoted)
- `$caller_tenant_id`, `$caller_user_id` → **host-bound tokens** (WS-10). Bound from `Principal`;
  **rejected if present in caller input.** This is the structural-isolation primitive.
**Security:** this is *the* injection boundary. All substituted values are quoted/escaped (or, better,
bound as real query parameters) for the target dialect; identifiers (column names in macros)
validated against an allowlist pattern. The engine never string-concatenates an untrusted value
unquoted. Host tokens can never be overridden by the caller. Unit-test the quoting + token-rejection hard.
**Freeze the signature in Wave 0** (incl. `params` + `host_tokens` for WS-10) so WS-01/02/10 can call it.

Wire `interpolate()` into the query path (`routes/query/run.rs` → store), driven by the new
`time_range` + variables fields on `QueryRequest`. Raw SQL with no macros passes through unchanged.

### B. Schema introspection
- `GET /api/v1/datasources/:id/schema` → `{ tables: [{ schema, name, columns: [{name, type}] }] }`
  for Postgres (query `information_schema`); cache per datasource (short TTL). Tenant + authz gated,
  read-only role.
- UI **schema browser** panel in the editor: tree of tables→columns, click-to-insert, search.

### C. Editor upgrade
- Replace textareas with **CodeMirror 6** + SQL language: syntax highlight, bracket match,
  schema-aware **autocomplete** (tables/columns from B), macro/variable highlighting, format button.
- Inline "Run" + result grid in the panel editor (couples with WS-04 preview) and in Explore.
- Surface `QueryStats` (rows, elapsed_ms, truncated) and column types already returned.

### D. Query history
- `0801_query_history.sql` (WS-03 `08xx` block): `{ id, tenant_id, user_id, datasource_id, sql, ran_at, elapsed_ms,
  row_count, error? }`, RLS-scoped. Write on each Explore/panel run (bounded retention).
- `GET /api/v1/query-history` + a history drawer in Explore (recall/re-run/star).

### E. Visual query builder (phase 2, can be its own follow-up session)
- Datasource→table→columns→aggregations→filters→group/order, generating SQL that round-trips to
  the text editor. Start with the SELECT/WHERE/GROUP-BY happy path for a single table.

## Design notes
- **Don't duplicate guards.** The macro engine produces SQL; the existing read-only/timeout/cap
  path (`nexus-store/src/query/run.rs`) still executes it. Macros change *text*, not governance.
- **Dialect-aware** from the start (even though only Postgres exists today) so WS-08 connectors can
  add their own time-bucket syntax — keep a small `Dialect` trait.
- DTO-first: extend `QueryRequest` in `nexus-spi`; regenerate OpenAPI + codegen.

## Acceptance criteria
- [ ] `interpolate()` handles every macro above with correct quoting; injection tests pass.
- [ ] A panel SQL with `$__timeFilter`/`$__timeGroup` runs correctly when WS-01 feeds a range.
- [ ] `$__sqlIn($region)` with a multi-value variable expands to a safe `IN (...)`.
- [ ] Schema endpoint returns tables/columns; the browser inserts identifiers into the editor.
- [ ] CodeMirror autocomplete suggests columns for the selected datasource.
- [ ] Query history records runs and supports recall/re-run; RLS-isolated across tenants.
- [ ] Tests mirrored backend + UI; live integration still green.

## Out of scope (hand off)
- Consuming the macros for time/vars → WS-01/WS-02 (this WS ships the engine + contract).
- Result caching → WS-09.
- New connector dialects → WS-08 (this WS leaves the `Dialect` seam ready).
