//! Spin up an ephemeral Postgres container and return a connected
//! [`crate::Pool`]. Requires Docker on the host.
//!
//! Pins `testcontainers` 0.23 + `testcontainers-modules` 0.11 — see
//! the crate `Cargo.toml` for the rationale.

use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;

use crate::pool::{connect, Pool};

/// Start a Postgres container, connect, and return a pool plus a
/// container handle.
///
/// **Drop the returned [`ContainerGuard`] last** — when it drops, the
/// container is torn down and the pool's connections become invalid.
/// The default image is the `postgres` module's pinned tag; default
/// credentials are `postgres` / `postgres`, database `postgres`.
///
/// Panics on container startup or connection failure (this is a test
/// helper, not a production API).
pub async fn with_database() -> (Pool, ContainerGuard) {
    let container = Postgres::default()
        .start()
        .await
        .expect("start postgres container");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("get container port");

    // The postgres-module image exposes the standard
    // postgres/postgres/postgres triple.
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = connect(&url).await.expect("connect to test postgres");

    (
        pool,
        ContainerGuard {
            _container: container,
        },
    )
}

/// RAII handle that stops the underlying container when dropped.
///
/// Hold this for the lifetime of the test. Dropping it before the
/// pool's last query has run will produce confusing connection-reset
/// errors.
pub struct ContainerGuard {
    _container: ContainerAsync<Postgres>,
}
