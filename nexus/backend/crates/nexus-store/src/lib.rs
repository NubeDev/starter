//! Nexus persistence — sqlx over Postgres.
//!
//! Two responsibilities live here: running user queries against a *datasource*
//! Postgres under the control-plane safety guards (read-only role, statement
//! timeout, forced limit, row/byte caps with pushdown), and — as the identity
//! milestone lands — the *metadata* store for datasources/dashboards/panels with
//! tenant Row-Level Security. The query path is sqlx-direct so the guards are
//! real connection/statement properties rather than engine config.

pub mod query;

pub use query::{run_query, QueryGuards};
