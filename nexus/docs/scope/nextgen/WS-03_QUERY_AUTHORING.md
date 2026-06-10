# WS-03 — Query & SQL Authoring (Macro Engine · Schema · Autocomplete · History · Builder)

> **Status:** Not started · **Wave:** 1 (ships the macro engine that WS-01/02 need) · **Owner:** _unassigned_
> **Depends on:** C4 OpenAPI conventions (Wave 0) · **Unblocks:** WS-01, WS-02
> **Migration:** block `08xx` (e.g. `0801_query_history.sql`) · **Read first:** GAP_ANALYSIS §2.3, ROADMAP §0 + §6 (C2)
> **Verified:** `fbf73a5c` on 2026-06-09 — re-grepped. **Drift found:** Scope B (schema introspection)
> and Scope C (CodeMirror editor) already shipped in the base commit `fbf73a5c` (see "Current state"
> below). The remaining WS-03 work is **A (the C2 binder)** and **D (query history)**.
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

## Current state (evidence — re-verified at `fbf73a5c`)
- **Scope B (schema introspection) — ALREADY SHIPPED.** `GET /api/v1/datasources/:id/schema`
  exists (`routes/datasources/schema.rs`), backed by `nexus_store::introspect`
  (`nexus-store/src/query/introspect.rs`) and the `DatasourceSchema`/`SchemaTable`/`SchemaColumn`
  DTOs (`nexus-spi/src/dto/datasource/schema.rs`). Tenant + `view`-authz + read-only guarded.
- **Scope C (CodeMirror editor) — ALREADY SHIPPED.** `ui/src/features/sql-editor/` has
  `SqlEditor.tsx` (CodeMirror 6 + `@codemirror/lang-sql`), `schemaCompletion.ts`,
  `useDatasourceSchema.ts` (schema-aware autocomplete), wired into `query-editor/Explore.tsx`.
- **Scope A (the C2 binder) — ABSENT.** No `bind()`/`BoundQuery`/`BindCtx`, no macro support
  anywhere in `nexus-store`/`nexus-engine`. `POST /api/v1/query` and
  `POST /api/v1/datasources/:id/query` both take `{ sql }` only (`nexus-spi/src/dto/query/run.rs`)
  and the runner is `sqlx::query(sql)` with no arg channel (`nexus-store/src/query/run.rs:44`).
- **Scope D (query history) — ABSENT.** No `query_history` table, store module, route, or UI.
- Result schema *is* returned (`ColumnSchema`) and shown in `Explore` results.

## Scope
### A. Macro / interpolation engine (C2 — the keystone, do first)
A single server-side entry point, e.g. `nexus-store/src/query/macro.rs`. **CRITICAL: it does NOT
return a finished SQL string** — returning `String` would force string-substitution at the project's
injection + tenant-isolation boundary. It returns **rewritten SQL with placeholders + the bound
argument vector + the set of identifiers it validated**, and the runner executes them as a
**prepared statement**:
```
fn bind(sql: &str, ctx: &BindCtx) -> Result<BoundQuery, BindError>

struct BoundQuery {
    sql:  String,              // rewritten with $1,$2,… placeholders — NO interpolated values
    args: Vec<SqlValue>,       // every value (time bounds, vars, kind params, host tokens) — BOUND, never inlined
    validated_identifiers: Vec<String>,  // the only strings ever inserted as text — each vetted (see below)
}
struct BindCtx {
    time_range: Option<TimeRange>,
    interval:   Option<Duration>,
    variables:  BTreeMap<String, VarValue>,   // WS-02 dashboard variables
    params:     BTreeMap<String, ParamValue>, // WS-10 kind named params (schema-validated upstream)
    host_tokens: HostTokens,                  // WS-10 host-bound: caller_tenant_id, caller_user_id
}
```
**The runner must change to accept the arg channel.** Today `nexus-store/src/query/run.rs:44` does
`sqlx::query(sql)` with **no arguments** — that's incompatible with binding. WS-03 updates the runner
to `sqlx::query_with(&bound.sql, bound.args)` (or `.bind()` each arg) so values are bound by the
driver, not concatenated. **This runner change is part of WS-03's scope, not an afterthought.**

This is the **single binder shared with [WS-10](./WS-10_KINDS_EXTENSIBILITY.md)** (C2). It serves
*both* raw-SQL macros and kind named-param binding — **do not build a second engine, and do not build
a string-substitution engine.**

Supported macros (Grafana-aligned):
- `$__timeFilter(col)` → `col >= '<from>' AND col < '<to>'`
- `$__timeGroup(col, '5m')` / `$__timeGroup(col, $__interval)` → dialect time bucket
- `$__timeFrom` / `$__timeTo` → literal timestamps
- `$__interval` → auto bucket (from WS-01's max_data_points) or the chosen interval var
- `$var`, `${var}`, `${var:csv}`, `${var:singlequote}`, `$__sqlIn(var)` → variable expansion
- `$<param>` → WS-10 kind named param → **always emitted as a bound `$N` arg** (never inlined)
- `$caller_tenant_id`, `$caller_user_id` → **host-bound tokens** (WS-10). Bound from `Principal`;
  **rejected if present in caller input.** This is the structural-isolation primitive.
**Security (this is *the* injection + tenant-isolation boundary — the contract above enforces it by
construction):**
- **Values are ALWAYS bound, never inlined.** Time bounds, `$var` values, `$__sqlIn` list elements,
  kind params, and host tokens (`$caller_tenant_id`) all go into `args` as `$N` placeholders. String
  quoting/escaping is **not** the primary defense and must never be the *only* one — prepared binding
  is mandatory. ("Safely quoted" language elsewhere in this doc means *bound*, not hand-escaped.)
- **The only text ever inserted into SQL is a *validated identifier/fragment*** — e.g. the column name
  in `$__timeFilter(col)` or a `$__timeGroup` bucket literal. Each is checked against a strict
  allowlist (identifier regex / enum of permitted fragments) and recorded in `validated_identifiers`.
  Values can't be bound as identifiers in SQL, so identifiers are the one unavoidable text path —
  keep it tiny and vetted.
- **Host tokens can never be overridden by the caller** (rejected if present in input).
- Unit-test: a `$var`/param containing `'); DROP …` lands as a bound arg and is inert; an identifier
  failing the allowlist is rejected; a caller-supplied `$caller_tenant_id` is rejected.
**Freeze the signature in Wave 0** (incl. `params` + `host_tokens` for WS-10) so WS-01/02/10 can call it.

Wire `bind()` into the query path (`routes/query/run.rs` → store), driven by the new `time_range` +
variables fields on `QueryRequest`, and **update the runner to execute `BoundQuery` as a prepared
statement** (replace `sqlx::query(sql)` at `run.rs:44` with arg-bound execution). Raw SQL with no
macros/params yields a `BoundQuery` with empty `args` and passes through unchanged.

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
- [ ] `bind()` returns `{sql, args, validated_identifiers}`; every value is a bound `$N` arg (none
  inlined); the runner executes it as a prepared statement; injection tests pass (malicious value →
  inert bound arg; bad identifier → rejected).
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
