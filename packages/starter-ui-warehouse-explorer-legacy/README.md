# `@nube/starter-ui-ch-explorer`

ClickHouse-only explorer UI, forked from
[`frectonz/sql-studio`](https://github.com/frectonz/sql-studio) (MIT).

The fork narrows the upstream SQLite/libSQL/MySQL/PostgreSQL/DuckDB
shell down to ClickHouse-only and rewires every API call against
the `starter-warehouse` explorer sub-router at
`/api/warehouse/ch/*` (see
`crates/starter-warehouse/src/explorer/mod.rs`).

## Scope

This package only ships the read-only explorer surface from PR 3 of
`rubix/docs/scope/clickhouse-explorer.md`:

- **Overview** — `GET /api/warehouse/ch/overview`
- **Tables list** — `GET /api/warehouse/ch/tables`
- **Table view** — `GET /api/warehouse/ch/tables/{name}` and
  `/data?page=N`
- **ERD** — `GET /api/warehouse/ch/erd`
- **Query editor** — `POST /api/warehouse/ch/query` (read-only;
  every write verb is refused server-side with HTTP 400)
- **Autocomplete** — `GET /api/warehouse/ch/autocomplete`

Marts, sandboxes, and `rubix.warehouse.*` verbs land in PR 4 and
are intentionally absent here.

## Differences from upstream

- API base path is `/api/warehouse/ch` (override at build time by
  setting a `<meta name="BASE_PATH" content="...">` tag in
  `index.html`).
- `POST /query` body is `{ "sql": value }` (upstream sends
  `{ "query": value }`).
- `metadata` and `shutdown` endpoints are dropped — server
  lifecycle belongs to `starter-server`, not the explorer.
- The `overview` schema drops `sqlite_version` and renames
  `db_size` → `size_on_disk` to match
  `crates/starter-warehouse/src/explorer/types.rs`.
- The upstream "Shutdown" button and SQL Studio GitHub link are
  removed from the header.

Every ported file carries a `// Forked from sql-studio (MIT)`
header. See `NOTICES.md` at the repo root for the full license
text and the list of derived paths.

## Develop

```bash
pnpm -F @nube/starter-ui-ch-explorer install
pnpm -F @nube/starter-ui-ch-explorer dev
# defaults: Vite serves the SPA on http://localhost:5173,
# proxies /api/warehouse/ch → http://localhost:3030
# override the backend with CH_EXPLORER_API_TARGET.
```

## Build

```bash
pnpm -F @nube/starter-ui-ch-explorer build
# emits ./dist with `base: /warehouse/explorer/`
# override with CH_EXPLORER_BASE
```

## Mount

The built `dist/` is served by `starter-server` via the static
asset rail introduced in PR 0:

```rust
use std::path::PathBuf;
use starter_server::ServerBuilder;

let dist = PathBuf::from("packages/starter-ui-ch-explorer/dist");
let app = ServerBuilder::new()
    .with_static_assets("/warehouse/explorer", dist)
    .build();
```

`/warehouse/explorer/*` then serves the SPA with `index.html` as
the SPA fallback, and `/api/warehouse/ch/*` is gated by
`starter-authz` with `("warehouse", "read")`.
