//! Nexus persistence — sqlx over Postgres.
//!
//! Two responsibilities live here: running user queries against a *datasource*
//! Postgres under the control-plane safety guards (read-only role, statement
//! timeout, forced limit, row/byte caps with pushdown), and the *metadata* store
//! for datasources/dashboards/panels with tenant Row-Level Security. The query
//! path is sqlx-direct so the guards are real connection/statement properties
//! rather than engine config; tenancy is RLS bound per transaction via
//! [`tenant_tx`].

pub mod agent;
pub mod alert;
pub mod changelog;
pub mod dashboard;
pub mod datasource;
pub mod flow;
pub mod folder;
pub mod migrate;
pub mod nav_node;
pub mod query;
pub mod query_history;
pub mod query_kind;
pub mod tag;
pub mod tenant_tx;
pub mod variable;

#[cfg(feature = "testing")]
pub mod testing;

pub use datasource::{Envelope, NewDatasource};
pub use query::{
    bind, bind_with, introspect, run_bound_query, run_kind_request, run_query, run_request, BindCtx,
    BindError, BoundQuery, ColumnInfo, Dialect, HostTokens, ParamValue, Postgres, QueryGuards,
    QueryIdentity, ScalarValue, SqlValue, TableInfo, TimeRange, VarValue,
};
