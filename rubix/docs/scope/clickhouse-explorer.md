# Plan — ClickHouse Explorer (sql-studio fork, CH-only)

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md). Source code must not
> reference this file — once a layer below lands, its design moves
> into `docs/design/warehouse/explorer/README.md` and code links there.

## What this plan is

Give rubix users an actual **ClickHouse explorer UI** by cherry-picking
from [`frectonz/sql-studio`](https://github.com/frectonz/sql-studio) (MIT)
both the ClickHouse backend module and the entire frontend, narrowed to
ClickHouse only, and grafting them onto our existing
`starter-store-clickhouse` / `starter-warehouse` stack — without
breaking the `forbid_raw_insert` write policy or the snapshot-before-write
contract of `rubix-tools/clickhouse`.

sql-studio's CH backend module is **half-stubbed** today
(`query` returns empty rows; `table_data` returns columns only,
no rows — `/tmp/sql-studio/src/main.rs` lines ~3925–4370). A pure
fork-and-run will not work. The plan is therefore "cherry-pick
the shape, finish the stubs, host on axum, point the UI at our routes."

## The single demo path

```
operator browser
       │  GET /warehouse/explorer/*  (served by starter-server)
       ▼
packages/starter-ui-ch-explorer        (fork of sql-studio/ui, CH-only)
   routes: /  /tables  /tables/:name  /schema  /query
   Monaco editor • @xyflow/react ERD • react-data-grid • shadcn
       │  fetch /api/warehouse/ch/*
       ▼
crates/starter-warehouse  (new axum sub-router: explorer::routes)
   GET  /api/warehouse/ch/overview               overview
   GET  /api/warehouse/ch/tables                 list with row counts
   GET  /api/warehouse/ch/tables/:name           sql + sizes
   GET  /api/warehouse/ch/tables/:name/data?page paged rows
   GET  /api/warehouse/ch/tables/:name/columns   columns + types
   GET  /api/warehouse/ch/erd                    tables + (synthetic) edges
   GET  /api/warehouse/ch/autocomplete           tables×columns
   POST /api/warehouse/ch/query   { sql }        read-only allow-list
       │
       ▼
starter-store-clickhouse::ChClient    (existing typed client)
   system.tables • system.columns • system.parts queries
       │
       ▼              ┌────────────────────────────────────────────┐
ClickHouse            │  WRITES never go through the explorer.     │
                      │  "Drop mart", "Set retention",             │
                      │  "Promote sandbox" buttons in the UI call  │
                      │  existing rubix verbs:                     │
                      │    rubix.clickhouse.mart.create            │
                      │    rubix.clickhouse.retention.set          │
                      │    rubix.clickhouse.rule.write             │
                      │  → snapshot-before-write + undo + changelog│
                      └────────────────────────────────────────────┘
```

## Layers exercised

| Layer | What lights up |
|---|---|
| Storage | `starter-store-clickhouse::ChClient` only — no new connection |
| HTTP | New `explorer` sub-module inside `starter-warehouse` axum router |
| Write policy | Read endpoints only; `POST /query` rejects non-`SELECT`/`SHOW`/`DESCRIBE`/`EXPLAIN`/`WITH` at the parser boundary |
| Frontend | New workspace package `packages/starter-ui-ch-explorer/` (Vite + TanStack Router) |
| Static serve | **No rail today.** `starter-server` only wires `/health`, `/metrics`, `openapi.json` — there is no `ServeDir` mount. See PR 0 below. |
| Rubix integration | UI surfaces existing rubix verbs as buttons; calls go to the existing tool dispatch, not new code |
| AuthN/Z | Reuse `starter-auth-users` session + `starter-authz` `warehouse.read` / `warehouse.write` permissions. Gating `/api/warehouse/ch/*` with `warehouse.read` is a PR 1 acceptance criterion, not optional. |
| Audit | **Explorer reads are not audited in PR 0–4.** There is no axum read-log middleware in the tree today; building one is a separate follow-up. Writes still carry the existing verb-level changelog because they go through `rubix.clickhouse.*` verbs, not through this router. |

## What is deliberately out

| Cut | Why |
|---|---|
| Multi-driver support (sqlite/pg/mysql/duckdb/parquet/csv/mssql/libsql) | sql-studio's selling point is "any DB"; ours is "ClickHouse done right." Strip everything except `mod clickhouse`. |
| sql-studio CLI shape (`sql-studio clickhouse --url …`) | We don't ship a separate binary; the explorer is a sub-router |
| Inline write UI (UPDATE/DELETE/ALTER from query box) | Violates `forbid_raw_insert`. Route through rubix verbs. |
| sql-studio's `/shutdown` endpoint | Server lifecycle is owned by `starter-server` |
| sql-studio metadata `version` / `can_shutdown` | Replace with our `/api/warehouse/status` payload |
| Forking sql-studio's `warp` server, `include_dir` static embed, `open` browser auto-launch | All replaced by existing `starter-server` infra |
| FK / ERD relationships (CH has no FKs) | Show columns + types only; no edges (consistent with sql-studio's CH `erd()` returning empty `relationships`). The `erd()` route still ships in PR 1 returning `{ tables: [...], relationships: [] }`; PR 3's UI degrades to "tables-as-nodes, no edges" — dagre still runs, just on disconnected nodes. |
| New crate | Keep this inside `starter-warehouse` to stay close to marts/sandboxes/dim_freshness; promote to its own crate only if it grows |

## What to cherry-pick from sql-studio backend

From `/tmp/sql-studio/src/main.rs`, `mod clickhouse` (lines ~3925–4370).
The **shapes of these queries** are the value — they encode CH's
`system.*` correctly:

- `overview()`: counts via `system.tables` (total / `engine = 'View'`),
  index proxy via `system.columns WHERE is_in_primary_key OR is_in_sorting_key`,
  per-table row counts + column counts grouped by `system.columns.table`.
- `table(name)`: `create_table_query` from `system.tables`,
  `formatReadableSize(sum(bytes))` from `system.parts`.
- `tables_with_columns()`: drives autocomplete.
- `erd()`: `system.columns ORDER BY position` with
  `is_in_primary_key` projected as `is_primary_key`.

**Do not** port the `Client::default().with_url(...)` open path —
inject the existing `ChClient` from `starter-store-clickhouse` so we
get one connection pool, one auth path, one TLS config.

**Finish the stubs** that sql-studio left empty:

- `table_data(name, page)`: currently returns columns only,
  empty rows. Implement with `SELECT … FROM <name> ORDER BY <first_col>
  LIMIT N OFFSET M`, marshalling rows via a dynamic `JSON FORMAT` round-trip
  (`SELECT … FORMAT JSONCompactEachRow` is the simplest CH-native path —
  no per-type Row codegen needed).
- `query(sql)`: ditto, plus the read-only allow-list described below.

## Read-only `POST /query` allow-list

No SQL parser dependency. Pure case-insensitive leading-token classifier
(no `sqlparser` in the workspace, and we're not adding one for this).
Accept exactly:

- `SELECT …`
- `WITH … SELECT …`
- `SHOW …` (databases / tables / columns / create / settings)
- `DESCRIBE` / `DESC`
- `EXPLAIN …`

Reject everything else with HTTP 400 `{"error":"read-only endpoint;
use rubix.clickhouse.* verbs for writes"}`. Enforce server-side, never
trust the UI to filter.

Defence-in-depth: execute under `SETTINGS readonly = 2`. Note that
`readonly = 2` permits `SET` (session-scoped settings changes), so a
caller can still flip `max_execution_time`, `max_memory_usage`, etc.
We accept that for v0 — the explorer is operator-only behind
`warehouse.read`. If/when we widen the audience, pin a dedicated CH
user with `readonly = 1` plus settings constraints; tracked as a
follow-up, not in this plan.

## What to fork from sql-studio UI

Copy `/tmp/sql-studio/ui/` into `packages/starter-ui-ch-explorer/`
verbatim, then narrow:

Keep:
- `src/api.ts` — rewrite `BASE_URL` to `/api/warehouse/ch`, drop
  `fetchMetadata` and `sendShutdown`, keep `fetchOverview`,
  `fetchTables`, `fetchTable`, `fetchTableData`, `fetchQuery`,
  `fetchAutocomplete`, plus the `erd*` schemas.
- `src/routes/{index,tables,schema,query}.tsx` and `routeTree.gen.ts`
  (regen with `tsr generate`).
- `src/components/` — Monaco wrapper, ERD (`@xyflow/react` + `dagre`),
  `react-data-grid` table, shadcn primitives, `sql-formatter` wiring.
- `src/lib/`, `src/provider/`, `src/main.tsx`, `src/index.css`.

Strip:
- Any per-driver branching (sqlite-version, dbstat, pragma references
  in UI copy).
- Shutdown button.
- "Open in browser" auto-launch logic (only lives in the Rust side
  anyway).

Add (rubix-specific):
- Left-tree sections for **Marts**, **Sandboxes**, **Cleaners** —
  feed from existing `/api/marts`, `/api/sandboxes`,
  `/api/warehouse/gc` endpoints.
- Overview tiles for **dim_freshness** (W11) and **W16 deltas** —
  feed from existing `/api/warehouse/status`.
- Action buttons that POST to existing rubix verb endpoints:
  - "Set retention" → `rubix.clickhouse.retention.set`
  - "Promote sandbox → mart" → `rubix.clickhouse.mart.create`
  - "Write insights rule" → `rubix.clickhouse.rule.write`

Build wiring follows `packages/starter-client-react/` — Vite build
into `dist/`, mounted by `starter-server` as static assets under
`/warehouse/explorer/*`.

## License hygiene

sql-studio is MIT. Every file copied from `/tmp/sql-studio/` keeps a
header:

```
// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
```

Add `NOTICES.md` at the workspace root with the full MIT text and
a list of paths derived from sql-studio.

## Five PRs

Each PR is independently shippable. Each one lights up another slice
of the explorer.

### PR 0 — static-asset mount in `starter-server`

Prerequisite for PR 3. `starter-server` today only wires `/health`,
`/metrics`, `openapi.json` — no `ServeDir`, no SPA fallback. Land
that rail first as its own PR so PR 3 is just "drop a built bundle
in the right place."

**Files touched:**
- `crates/starter-server/src/static_assets.rs` — new module exposing
  `fn mount(router, mount_path, dist_dir) -> Router` built on
  `tower_http::services::ServeDir` with SPA `fallback` to `index.html`.
- `crates/starter-server/src/server_builder.rs` — call site (gated
  off by default; opt-in per binary).
- `crates/starter-server/Cargo.toml` — `tower-http` with `fs` feature.
- `crates/starter-server/tests/static_assets_test.rs` — serves a
  temp dir, asserts `index.html` fallback and asset MIME types.

**Exit signal.** A throwaway binary can mount a directory of HTML at
`/anything/*` and SPA-route fallback to `index.html`.

**Alternative if PR 0 slips:** PR 3 ships only a Vite dev-server
proxy (`vite.config.ts` → `/api/warehouse/ch` → backend port) and
explicitly defers production hosting. Do not pretend the rail exists.

### PR 1 — backend skeleton, no UI

The minimal vertical: hit `GET /api/warehouse/ch/tables` from `curl`
and get real rows.

**Files touched:**
- `crates/starter-warehouse/src/explorer/mod.rs` — new module;
  `routes(ch: ChClient) -> axum::Router`.
- `crates/starter-warehouse/src/explorer/queries.rs` — port the
  six `system.*` queries from sql-studio's `mod clickhouse`,
  rewritten against `ChClient`.
- `crates/starter-warehouse/src/explorer/types.rs` — port
  `Overview`, `Tables`, `Table`, `TableData`, `Erd*`, `Count`,
  `TablesWithColumns` from sql-studio `mod responses` (drop
  `sqlite_version`, rename `db_size` → `size_on_disk`).
- `crates/starter-warehouse/src/lib.rs` — mount `explorer::routes`
  at `/api/warehouse/ch`; ensure the existing `starter-authz` layer
  applied to `/api/warehouse/*` requires `warehouse.read` on every
  `GET /api/warehouse/ch/*`.
- `crates/starter-warehouse/src/explorer/mod.rs` — top-of-file
  comment header pointing at this scope doc *with a TODO* to flip
  the link to `docs/design/warehouse/explorer/README.md` in PR 4.
  (HOW-TO-CODE.md §0a forbids code referencing scope; this header
  is an explicit waiver because the design doc lands in PR 4 — call
  it out in the PR description.)
  Alternative: land a stub `docs/design/warehouse/explorer/README.md`
  in PR 1 and point at that from day one. Pick one before opening PR 1.
- `crates/starter-warehouse/tests/explorer_smoke.rs` — bring up a
  CH testcontainer (the warehouse crate already uses one), assert
  `/tables` returns the expected mart shells; assert
  `/api/warehouse/ch/tables` requires `warehouse.read` (401 without
  session, 403 with session lacking the permission).
- `crates/starter-warehouse/tests/explorer_no_write_path.rs` —
  structural test that the explorer module's public API exposes
  no write surface (e.g. compile-fail snippet that any attempt to
  call an `INSERT`/`ALTER` path on the explorer router fails to
  build, or trybuild-style assertion).
- **New** `NOTICES.md` at workspace root (does not exist today;
  create alongside existing `LICENSE`, don't disturb it).

**Exit signal.** `curl :PORT/api/warehouse/ch/tables` returns JSON
with non-stub rows. `cargo test -p starter-warehouse --test
explorer_smoke` passes, including the authz gate assertion. The
explorer router has no compile-time path to a write surface.

### PR 2 — read-only `POST /query` + `table_data`

Finish sql-studio's stubs. This is the part that didn't exist
upstream.

**Blast radius note.** `ChClient` today only exposes typed
`.fetch_all::<Row>()` over the `clickhouse` crate — no raw HTTP
escape hatch. PR 2 therefore **does** touch `starter-store-clickhouse`
to add a `ChClient::fetch_json(sql) -> serde_json::Value` (or
`fetch_rows_dynamic`) that runs `SELECT … FORMAT JSONCompactEachRow`
over the underlying HTTP transport and parses the streamed JSON.
This is the smallest addition that keeps dynamic-shape queries out
of the typed Row path; gate it behind `pub(crate)` + a feature flag
if `forbid_raw_insert` reviewers want belt-and-braces.

**Files touched:**
- `crates/starter-store-clickhouse/src/raw.rs` — new
  `ChClient::fetch_json` (read-only HTTP `POST` to CH with
  `query=… FORMAT JSONCompactEachRow`, returns parsed columns+rows).
  Doctest asserts it rejects anything starting with a write verb
  before sending, preserving `forbid_raw_insert` intent.
- `crates/starter-warehouse/src/explorer/queries.rs` — implement
  `table_data` and `query()` via the new `fetch_json`, executed
  under `SETTINGS readonly = 2`.
- `crates/starter-warehouse/src/explorer/parse.rs` — leading-token
  classifier; unit tests for every accepted / rejected shape
  (`INSERT`, `ALTER`, `OPTIMIZE`, `TRUNCATE`, `KILL`, `SYSTEM`,
  `CREATE`, `DROP`, `RENAME`, `ATTACH`, `DETACH` all rejected).
- `crates/starter-warehouse/tests/explorer_query_test.rs` —
  round-trip `SELECT count() FROM rubix_dim_freshness` and one
  rejected `DROP TABLE`.

**Exit signal.** A `POST /api/warehouse/ch/query` with `SELECT 1`
returns `{"columns":["1"],"rows":[[1]]}`. The same endpoint with
`DROP TABLE x` returns 400. CH server-side `readonly = 2` blocks
anything that slipped through.

### PR 3 — UI fork, ClickHouse-only

Get the sql-studio frontend on screen, pointed at our routes.
No rubix-specific overlays yet.

**Files touched:**
- `packages/starter-ui-ch-explorer/` — new workspace package;
  copy `ui/src/` from sql-studio with MIT headers; narrow per the
  "What to fork" section.
- `packages/starter-ui-ch-explorer/package.json`,
  `vite.config.ts`, `tsconfig.json` — follow
  `packages/starter-client-react/` patterns.
- `pnpm-workspace.yaml` — add the new package.
- `crates/starter-server/src/static_assets.rs` (or wherever
  client-react is mounted) — add a mount at `/warehouse/explorer`.
- `crates/starter-warehouse/src/explorer/mod.rs` — add the
  missing `autocomplete` route the UI calls on boot.

**Exit signal.** Browser hits `/warehouse/explorer`, sees the
sql-studio shell with our database name. Tables list, schema, ERD,
and Monaco query editor all work against the CH testcontainer.

### PR 4 — rubix overlays (marts, sandboxes, verbs)

The reason this explorer exists in a rubix tree instead of just
running sql-studio: surface our own constructs.

**Files touched:**
- `packages/starter-ui-ch-explorer/src/components/rubix/`:
  `MartTree.tsx`, `SandboxTree.tsx`, `CleanerTree.tsx`,
  `FreshnessTiles.tsx`, `ActionButtons.tsx`.
- `packages/starter-ui-ch-explorer/src/routes/index.tsx` — splice
  in `FreshnessTiles` + W16 deltas next to sql-studio's counts
  panel.
- `packages/starter-ui-ch-explorer/src/api.ts` — add typed wrappers
  for `/api/marts`, `/api/sandboxes`, `/api/warehouse/{status,gc,audit}`,
  and the three rubix verb POST shapes.
- New `docs/design/warehouse/explorer/README.md` (placeholder →
  present-tense once this PR lands). Code references this, not the
  scope file.

**Exit signal.** Operator can: see all marts grouped under
"Marts", click "Set retention 30d" on a mart, see the changelog row
land, see dim_freshness tile update on next refresh. No raw
INSERT/ALTER ever leaves the browser without going through a typed
verb endpoint.

## Open questions

1. **Crate boundary.** Keep inside `starter-warehouse` (this plan's
   choice) or split out a `starter-warehouse-explorer` crate? Split
   only if it grows past ~1500 LoC or pulls in heavy deps.
2. **UI package location.** `packages/starter-ui-ch-explorer/`
   (this plan's choice) vs co-located with the eventual SDUI app.
   First-pass keeps it standalone, mountable from any binary.
3. ~~**Auth.**~~ Resolved: promoted out of "open" into PR 1's
   acceptance criteria. `warehouse.read` gates every
   `/api/warehouse/ch/*` route, enforced by `starter-authz`. No
   anonymous reads, period.
4. **Allow-list parser.** Cheap leading-token check (this plan's
   choice). Revisit only if CTEs / hints / per-shard
   `SELECT … INTO OUTFILE …` slip through; no `sqlparser` dep
   today and none planned.
5. **`JSONCompactEachRow` cost.** Acceptable for an explorer
   (per-page, 100s of rows). If we ever stream millions, swap to
   `RowBinaryWithNamesAndTypes` and a small in-house decoder.
6. **Sample data.** sql-studio ships a `sample.db`; we should
   provide a seed task that loads the same fixture marts used by
   `starter-warehouse` tests, so the explorer is non-empty on a
   fresh dev box. Task runner: `mani` per `rubix/Makefile`
   (`mani run warehouse-seed`), exposed as a `make warehouse-seed`
   target for parity with other dev workflows.
