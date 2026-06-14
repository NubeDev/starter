//! The guarded one-shot query path against a datasource Postgres.

mod bind;
mod introspect;
mod request;
pub mod row_json;
mod run;

use std::time::Duration;

pub use bind::{
    bind, bind_with, BindCtx, BindError, BoundQuery, Dialect, HostTokens, ParamValue, Postgres,
    ScalarValue, SqlValue, TimeRange, VarValue,
};
pub use introspect::{
    introspect, introspect_tenant_ro, ColumnInfo, RelationInfo, SchemaInfo, TableInfo,
};
pub use request::{run_kind_request, run_request, QueryIdentity};
pub use row_json::{columns_of, row_to_object};
pub use run::{run_bound_query, run_query, run_query_tenant_ro};

/// The server-enforced safety bounds applied to every datasource query. None of
/// these are expressible by the caller — they are set from server config and the
/// datasource's policy, never from the request body.
#[derive(Debug, Clone, Copy)]
pub struct QueryGuards {
    /// Postgres `statement_timeout` for the query transaction.
    pub statement_timeout: Duration,
    /// Maximum rows to return before truncating.
    pub max_rows: u64,
    /// Maximum serialized bytes to return before truncating.
    pub max_bytes: u64,
}
