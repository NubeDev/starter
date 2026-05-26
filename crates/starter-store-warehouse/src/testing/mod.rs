//! Testcontainer factory. `feature = "testing"`.
//!
//! Mirrors `starter_store_postgres::testing::with_database` — spin
//! up an ephemeral ClickHouse container, build a [`ChClient`] with
//! the W8 settings, return both plus an RAII container guard.

use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::clickhouse::ClickHouse as ClickHouseImage;

use crate::client::{ChClient, ChConfig};

/// Start a ClickHouse container, connect via HTTP, return the
/// client plus a guard. Drop the guard last; dropping it stops
/// the container.
pub async fn with_clickhouse() -> (ChClient, ContainerGuard) {
    let container = ClickHouseImage::default()
        .start()
        .await
        .expect("start clickhouse container");

    let port = container
        .get_host_port_ipv4(8123)
        .await
        .expect("get container HTTP port");

    let url = format!("http://127.0.0.1:{port}");
    let client = ChClient::connect(ChConfig::local(url));

    (
        client,
        ContainerGuard {
            _container: container,
        },
    )
}

pub struct ContainerGuard {
    _container: ContainerAsync<ClickHouseImage>,
}
