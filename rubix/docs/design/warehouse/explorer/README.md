# Warehouse Explorer (design)

Present-tense reference for `crates/starter-warehouse/src/explorer/`
and `packages/starter-ui-ch-explorer/`. The forward-looking plan and
PR sequencing live in
[`rubix/docs/scope/clickhouse-explorer.md`](../../scope/clickhouse-explorer.md);
this page documents what the code already does so source files can
satisfy [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) (code may
reference `docs/design/`, never `docs/scope/`).

## What it is

A read-only HTTP sub-router under `/api/warehouse/ch/*` plus a
TanStack-Router SPA fork of
[`frectonz/sql-studio`](https://github.com/frectonz/sql-studio)
(MIT, ClickHouse-only). The sub-router lives in
`crates/starter-warehouse/src/explorer/`; the SPA lives in
`packages/starter-ui-ch-explorer/`. Both are mounted end-to-end by
[`examples/ch-explorer/`](../../../../examples/ch-explorer/).

## Public HTTP surface

```text
GET  /api/warehouse/ch/overview                size_on_disk + counters
GET  /api/warehouse/ch/tables                  table list + row counts
GET  /api/warehouse/ch/tables/{name}           SQL + sizes + counts
GET  /api/warehouse/ch/tables/{name}/data      paged rows (?page=N, 50/page)
GET  /api/warehouse/ch/tables/{name}/columns   columns + types
GET  /api/warehouse/ch/erd                     tables + synthetic edges
GET  /api/warehouse/ch/autocomplete            tables × columns
POST /api/warehouse/ch/query   { sql }         read-only allow-list
```

The sub-router is built by
[`explorer::routes`](../../../../crates/starter-warehouse/src/explorer/mod.rs)
and gated at the mount site with
[`starter_authz::with_permission(router, "warehouse", "read")`](../../../../crates/starter-authz/src/middleware.rs).
Unauthenticated requests return `401`; authenticated requests
without the `warehouse.read` permission return `403`; both
behaviours are asserted in
[`tests/explorer_smoke.rs`](../../../../crates/starter-warehouse/tests/explorer_smoke.rs).

## Storage seam

Every read goes through the existing
[`ChClient`](../../../../crates/starter-store-warehouse/src/client.rs)
from `starter-store-warehouse`. The explorer opens no second
connection of its own, so the W8 `async_insert` discipline and the
client's `database` / `user` settings apply uniformly.

Dynamic-shape SELECTs (the `POST /query` body and the per-table
`tables/{name}/data` pagination) flow through
[`ChClient::fetch_json`](../../../../crates/starter-store-warehouse/src/raw.rs),
which posts `… FORMAT JSONCompactEachRow` over the underlying HTTP
transport. The function is the only read-side escape hatch on
`ChClient`; it refuses any input whose leading non-comment token
isn't a `SELECT`/`SHOW`/`DESCRIBE`/`EXPLAIN`/`WITH`, so a
compromised explorer route still cannot reach a write verb.

## Write-safety

Three independent layers refuse writes through the explorer:

1. **Router shape.** `explorer::routes` exposes one mutating verb
   (`POST /query`) and no `PUT`/`PATCH`/`DELETE`. The structural
   no-write-path assertion lives in
   [`tests/explorer_no_write_path.rs`](../../../../crates/starter-warehouse/tests/explorer_no_write_path.rs).
2. **Statement parser.**
   [`explorer::parse::classify`](../../../../crates/starter-warehouse/src/explorer/parse.rs)
   is a cheap leading-token allow-list. Anything other than
   `SELECT`/`SHOW`/`DESCRIBE`/`EXPLAIN`/`WITH` is rejected with
   HTTP `400` and `{"error":"read_only_violation"}`. Unit tests
   cover the full ClickHouse mutating-verb vocabulary
   (`INSERT`, `ALTER`, `OPTIMIZE`, `TRUNCATE`, `KILL`, `SYSTEM`,
   `CREATE`, `DROP`, `RENAME`, `ATTACH`, `DETACH`).
3. **`ChClient::fetch_json`** repeats the same leading-token check
   before talking to ClickHouse, so a bug in the parser fails
   closed.

The ClickHouse session itself runs `POST /query` shapes under
`SETTINGS readonly = 2`, which is the server-side belt to the
client-side braces.

## SPA hosting

The SPA's production base path is `/warehouse/explorer/`
(overridable at build time with `CH_EXPLORER_BASE`). The
`starter-server::ServerBuilder` exposes
`.with_static_assets(mount_path, dist_dir)`
([`crates/starter-server/src/static_assets.rs`](../../../../crates/starter-server/src/static_assets.rs)),
which wraps `tower_http::services::ServeDir` with an `index.html`
fallback so the client-side router owns the URL space below the
mount. The example binary calls

```rust
ServerBuilder::<()>::new(())
    .merge_router(with_permission(
        starter_warehouse::explorer::routes(ch),
        "warehouse",
        "read",
    ))
    .with_static_assets("/warehouse/explorer", dist)
    .build();
```

For local development, `pnpm -F @nube/starter-ui-ch-explorer dev`
serves the SPA on `:5173` and proxies `/api/warehouse/ch/*` to
`CH_EXPLORER_API_TARGET` (default `http://localhost:3030`, which
matches the example binary's bind).

## Authz

`warehouse.read` is the only permission the explorer requires.
The example binary at [`examples/ch-explorer/`](../../../../examples/ch-explorer/)
mounts a dev-only `AllowAll` engine + anonymous `Role::Admin`
principal so the gate is satisfied without a login flow. Production
binaries pair `with_principal(authenticator)` with a `DbPolicyEngine`
and grant `("warehouse","read")` to the relevant role — see
[`examples/authz-demo/src/server.rs`](../../../../examples/authz-demo/src/server.rs)
for the full shape.

## Audit posture

Explorer reads are **not** audited by the router itself. Writes
never go through the explorer — every mutation (mart create,
retention set, projection-rule write, sandbox lifecycle) flows
through the `rubix.warehouse.*` verbs registered in
[`rubix/crates/rubix-agent/src/registry.rs`](../../../../rubix/crates/rubix-agent/src/registry.rs),
each of which already carries snapshot-before-write + undo +
changelog.

## Fork hygiene

Every file ported from sql-studio carries a
`// Forked from sql-studio (MIT)` header. The list of derived paths
and the full MIT text live in
[`NOTICES.md`](../../../../NOTICES.md) at the workspace root.
The workspace [`LICENSE`](../../../../LICENSE) is untouched.

## Frontend topology

The explorer ships as a **headless React component library**:

- [`@nube/starter-ui-ch-explorer`](../../../../packages/starter-ui-ch-explorer/)
  exports `<Explorer />` (tabbed shell), the four individual views
  (`<ExplorerOverview />`, `<ExplorerTables />`, `<ExplorerSchema />`,
  `<ExplorerQuery />`), and a set of typed React Query hooks
  (`useChOverview`, `useChTables`, `useChTable`, `useChTableData`,
  `useChQuery`, `useChErd`, `useChAutocomplete`). The library
  declares no `main.tsx`, no router, no `QueryClient`, no theme
  provider, and no transport — hosts wire those up. Visible strings
  flow through `DEFAULT_EXPLORER_MESSAGES` + `<ExplorerI18nProvider>`
  on the same pattern as `@nube/starter-ui-authz`.
- Two **rubix-only overlays** live behind the optional
  [`./rubix` subpath](../../../../packages/starter-ui-ch-explorer/src/rubix/):
  `<FreshnessTiles />` reads `/api/warehouse/status` via
  `@nube/starter-client-ts`'s typed `fetchJson` (returns `null` on
  `404`/`503` so it stays invisible on the explorer-only demo
  binary); `<MartTree />` lists + drops marts through the typed
  `useClickhouseMartsList` / `useClickhouseMartDrop` hooks from
  `@nube/rubix-client-react`, surfaces destructive UX through the
  kit's `<AlertDialog>` (no `window.confirm`), and is gated behind
  an optional peer dep so the demo host doesn't pull rubix in.

The library mounts inside two hosts:

| Host | What it adds | Where |
|---|---|---|
| **rubix shell** (primary) | full rubix chrome, `react-intl` catalogue, theme tokens, sidebar / breadcrumbs; composes `<FreshnessTiles />` + `<MartTree />` above `<Explorer />` | `rubix/frontend/src/components/admin/warehouse/explorer-panel.tsx`, mounted as the fifth tab of `<WarehouseAdmin>` at `/admin/warehouse` |
| **demo binary** (fallback) | thin Vite host: `QueryClientProvider` + `StarterClientProvider` + tailwind tokens, `<Explorer />` only | `examples/ch-explorer/ui/`, served by `cargo run -p ch-explorer-example -- serve` under `/warehouse/explorer/` |

One library, two mount points. The destructive-write contract
(snapshot + undo + changelog) is preserved because every mutation
goes through `POST /api/v1/tools/{tool_id}` on rubix-agent, never
through a bespoke transport in the library.
