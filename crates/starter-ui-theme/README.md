# starter-ui-theme

Backend half of the theme editor documented in
[../../DOCS/frontend/theme/README.md](../../DOCS/frontend/theme/README.md).
The frontend ships a transport-agnostic editor; this crate ships the
six REST routes its default `httpThemeTransport` calls, plus
[`ThemeStore`](../starter-spi/src/ui/theme/store.rs) implementations
for sqlite and Postgres.

## What's in the box

| Module | Feature flag | Purpose |
|---|---|---|
| `store::SqliteThemeStore` | `sqlite` | Single-row table in the consumer's sqlite pool. |
| `store::PostgresThemeStore` | `postgres` | Same shape in Postgres (`JSONB` + `BYTEA`). |
| `routes::theme_router` | `routes` (default) | The six handlers as an `axum::Router`. |
| `openapi::openapi()` | `routes` | utoipa document for the routes — merge into the consumer's served doc. |

Migrations ship under `migrations/ui_theme_sqlite/` and
`migrations/ui_theme_postgres/` — both use the source name
`ui_theme` for the namespaced runner.

## Wiring it into a consumer

```rust
use std::sync::Arc;
use starter_server::auth::with_principal;
use starter_store_sqlite::{migrate, migrate::MigrationSource};
use starter_ui_theme::{routes::{theme_router, ThemeState}, store::SqliteThemeStore};

static UI_THEME_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-ui-theme/migrations/ui_theme_sqlite");

// 1. Apply migrations alongside your other sources.
migrate(&pool)
    .with_source(MigrationSource { name: "ui_theme", migrator: &UI_THEME_MIGRATOR })
    // .with_source(...your other sources)
    .run().await?;

// 2. Build the router and wrap with the authenticator so handlers
//    see the `Principal` extension.
let store = Arc::new(SqliteThemeStore::new(pool.clone()));
let router = theme_router::<AppState>(ThemeState::new(store));
let router = with_principal(router, authenticator.clone());

// 3. Merge into the rest of the app.
server_builder.merge_router(router);
```

## Authorisation contract

| Route | Auth |
|---|---|
| `GET /api/v1/ui/theme` | Any authenticated principal |
| `PUT /api/v1/ui/theme` | `Role::Admin` |
| `POST/DELETE /api/v1/ui/theme/logo` | `Role::Admin` |
| `POST/DELETE /api/v1/ui/theme/favicon` | `Role::Admin` |
| `GET /api/v1/ui/theme/{logo,favicon}` | **Public** — browsers can't ride a cookie/bearer header on `<img src>` / favicon-link requests. The bytes are non-sensitive (the org's public logo). |

Validation guards mutations: any token value containing `url(`,
`@import`, `expression(` or `javascript:` is rejected with a
[`Problem`](../starter-spi/src/dto/problem.rs) `400` listing the
offending key. The same check lives in `starter_spi::ui::theme::
validate_save_input` so other transports get the same denylist.

## Asset storage

Assets are stored inline as BLOBs in the same single-row table the
styles live in. Trade-off vs. the README's "filesystem-backed asset
dir":

- **+ No filesystem coupling** — round-trip tests stay trivial,
  read-only deploys work, multi-replica deploys don't need a shared
  volume.
- **+ No separate static-files route to wire** — the `GET` handlers
  in this crate serve the bytes with the stored MIME directly.
- **− Backups now carry the asset bytes** — fine at the size caps
  the README promises (256 KiB logo / 64 KiB favicon).

S3 / MinIO can land as a parallel `ThemeStore` impl when a consumer
asks, per the TODO Phase 9b note.

## OpenAPI snapshot

`tests/openapi_snapshot.rs` locks the document to
`DOCS/backend/openapi/ui-theme.openapi.json`. Refresh after route
signature changes:

```
UPDATE_SNAPSHOTS=1 cargo test -p starter-ui-theme --test openapi_snapshot
```

## Running the tests

```
cargo test -p starter-ui-theme --features sqlite,postgres
```

15 tests cover: SPI compile / object-safety (in `starter-spi`),
sqlite store round-trips, route auth, validator rejection, asset
size + MIME enforcement, and the OpenAPI snapshot.
