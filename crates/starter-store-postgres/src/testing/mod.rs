//! Testcontainers-backed test pool factory. `feature = "testing"`.

mod with_database;

pub use with_database::{with_database, ContainerGuard};
