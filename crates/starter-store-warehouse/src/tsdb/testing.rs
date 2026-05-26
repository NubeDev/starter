//! Spin up an ephemeral TimescaleDB container for tests.
//!
//! Uses the `timescale/timescaledb-ha:pg16` image — the same one
//! the dev compose stack pins (`docker/docker-compose.warehouse.yml`).
//! The Postgres testcontainers module's `Postgres::with_image` API
//! is not stable enough across the 0.11 release line to rely on,
//! so this helper builds a `GenericImage` directly.

use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

use super::client::WarehouseClient;

/// Start a TimescaleDB container, connect via `sqlx`, and return
/// a [`WarehouseClient`] plus an RAII guard. Drop the guard last
/// — when it drops the container is stopped and connections
/// become invalid.
pub async fn with_timescale() -> (WarehouseClient, TimescaleGuard) {
    let container = GenericImage::new("timescale/timescaledb-ha", "pg16")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_USER", "warehouse")
        .with_env_var("POSTGRES_PASSWORD", "warehouse")
        .with_env_var("POSTGRES_DB", "warehouse")
        .start()
        .await
        .expect("start timescaledb container");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("get container port");

    let url = format!("postgres://warehouse:warehouse@127.0.0.1:{port}/warehouse");
    let client = WarehouseClient::connect(&url)
        .await
        .expect("connect to timescaledb");

    (
        client,
        TimescaleGuard {
            _container: container,
        },
    )
}

/// Holds the container alive for the duration of the test.
pub struct TimescaleGuard {
    _container: ContainerAsync<GenericImage>,
}
