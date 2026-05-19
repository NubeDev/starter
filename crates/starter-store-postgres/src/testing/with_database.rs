//! Spin up an ephemeral Postgres container and return a connected
//! [`crate::Pool`]. Requires Docker on the host.

use crate::pool::Pool;

/// Start a Postgres container, connect, and return a pool plus a
/// handle that keeps the container alive.
///
/// Drop the returned tuple to tear down the container.
///
/// Stubbed for v0.1 — landing the actual testcontainers wiring is
/// blocked on picking between `testcontainers` 0.x and 0.20+.
pub async fn with_database() -> (Pool, ContainerGuard) {
    todo!("testcontainers integration lands in v0.2")
}

/// RAII guard that stops the underlying container when dropped.
pub struct ContainerGuard {
    _private: (),
}
