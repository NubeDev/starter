//! Nexus persistence — sqlx over Postgres.
//!
//! Two responsibilities live here: running user queries against a *datasource*
//! Postgres under the control-plane safety guards (read-only role, statement
//! timeout, forced limit, row/byte caps with pushdown), and the *metadata* store
//! for datasources/dashboards/panels with tenant Row-Level Security. The query
//! path is sqlx-direct so the guards are real connection/statement properties
//! rather than engine config; tenancy is RLS bound per transaction via
//! [`tenant_tx`].

pub mod dashboard;
pub mod datasource;
pub mod migrate;
pub mod query;
pub mod tenant_tx;

#[cfg(feature = "testing")]
pub mod testing;

pub use datasource::{Envelope, NewDatasource};
pub use query::{run_query, QueryGuards};
