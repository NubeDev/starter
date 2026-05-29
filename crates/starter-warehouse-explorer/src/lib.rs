//! starter-warehouse-explorer — read-only REST explorer surface over
//! the TimescaleDB-backed warehouse pool.
//!
//! Phase 4 of `rubix/docs/proposal/warehouse-engine-swap.md` restores
//! the sql-studio-style admin surface that was over-deleted in
//! phase 3 alongside the ClickHouse engine. The wire shape is
//! preserved verbatim so the existing frontend reviver in
//! `packages/starter-ui-warehouse-explorer` keeps working; the URL
//! prefix moves from the legacy `/api/warehouse/ch/*` to the
//! vendor-neutral `/api/warehouse/explorer/*`.
//!
//! Seven endpoints, all `Role::Admin`-gated when mounted via
//! [`router_with_auth`]:
//!
//! | Method | Path                                            |
//! |--------|-------------------------------------------------|
//! | GET    | `/api/warehouse/explorer/overview`              |
//! | GET    | `/api/warehouse/explorer/tables`                |
//! | GET    | `/api/warehouse/explorer/tables/{name}`         |
//! | GET    | `/api/warehouse/explorer/tables/{name}/data`    |
//! | POST   | `/api/warehouse/explorer/query`                 |
//! | GET    | `/api/warehouse/explorer/autocomplete`          |
//! | GET    | `/api/warehouse/explorer/erd`                   |
//!
//! ## Safety
//!
//! `POST /query` accepts arbitrary SQL. The only safe gate is the
//! engine itself — every statement runs inside a
//! `BEGIN READ ONLY DEFERRABLE` transaction with a hard
//! `statement_timeout`. Mutations (`INSERT`/`UPDATE`/`DELETE`/`DDL`)
//! are rejected by Postgres with `25006 read_only_sql_transaction`.
//! We do **no** string-level SQL parsing.
//!
//! Production deployments should additionally point this crate at a
//! dedicated low-privilege Postgres role with `SELECT`-only grants on
//! `public` and the `timescaledb_information` views — a defence in
//! depth against engine bugs and any future relaxation of the
//! transaction's read-only enforcement.
//!
//! ## Identifier validation
//!
//! Path parameters (`/tables/{name}`) are validated against
//! `^[A-Za-z_][A-Za-z0-9_]*$` *before* any SQL is issued.
//! Schema-qualified names are rejected — `public` is implicit.

use std::sync::Arc;

use axum::Router;
use starter_server::auth::{with_principal, with_role};
use starter_spi::auth::{Authenticator, Role};
use starter_store_warehouse::WarehouseClient;

pub mod handlers;
pub mod queries;
pub mod types;
pub mod validate;

/// Mount path prefix. Phase 4 also completes the vendor-neutral
/// rename of the URL — the old surface lived at
/// `/api/warehouse/ch/*`; the `ch` token is gone now that the
/// backend is Postgres/TimescaleDB. The frontend `API_ROOT`
/// constant in `use-warehouse.ts` is kept in lock-step.
pub const MOUNT_PREFIX: &str = "/api/warehouse/explorer";

/// Per-statement timeout applied to the `POST /query` transaction.
/// 90s leaves enough headroom for cold portfolio-scale aggregates
/// (e.g. `usage_per_meter` against ~700 meters × 1 year of
/// histories, which can take ~30s on a cold buffer cache) while
/// still preventing a runaway from monopolising a pool connection.
pub const QUERY_STATEMENT_TIMEOUT_MS: u32 = 90_000;

/// Server-enforced page size for `/tables/{name}/data?page=N`.
/// Matches the upstream sql-studio constant — the client controls
/// only the page index.
pub const PAGE_SIZE: i64 = 50;

/// Shared handler state — the warehouse pool plus a tiny in-process
/// autocomplete cache.
#[derive(Clone)]
pub struct ExplorerState {
    pub client: WarehouseClient,
    pub autocomplete_cache: Arc<tokio::sync::Mutex<Option<CachedAutocomplete>>>,
}

/// Cache entry for `/autocomplete`. The endpoint is hot (Monaco
/// fires it on every editor mount) and the underlying
/// `information_schema.columns` scan is non-trivial on a
/// hypertable-heavy database.
#[derive(Clone)]
pub struct CachedAutocomplete {
    pub fetched_at: std::time::Instant,
    pub payload: types::Autocomplete,
}

impl ExplorerState {
    pub fn new(client: WarehouseClient) -> Self {
        Self {
            client,
            autocomplete_cache: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

/// Build the explorer router **without** auth — suitable for tests.
/// Production callers must use [`router_with_auth`].
pub fn router(client: WarehouseClient) -> Router {
    let state = ExplorerState::new(client);
    Router::new()
        .nest(MOUNT_PREFIX, handlers::routes())
        .with_state(state)
}

/// Build the explorer router gated by
/// `with_principal` → `with_role(Role::Admin)`. Mirrors the gate the
/// pre-phase-3 explorer used and matches `starter-ext-server`'s
/// admin surface.
pub fn router_with_auth<A>(client: WarehouseClient, authenticator: Arc<A>) -> Router
where
    A: Authenticator + ?Sized,
{
    let inner = router(client);
    with_principal(with_role(inner, Role::Admin), authenticator)
}
