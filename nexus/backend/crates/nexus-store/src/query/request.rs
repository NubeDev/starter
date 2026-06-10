//! Map a wire `QueryRequest` plus the caller's identity into a [`BindCtx`],
//! bind it, and run it under the guards.
//!
//! This keeps the transport handler thin: it hands the request, the
//! principal-derived host tokens, and the pool here, and gets rows back. The
//! handler never touches the binder or the runner directly. See
//! docs/design/query/.

use std::collections::BTreeMap;
use std::time::Duration;

use nexus_spi::dto::query::{QueryRequest, QueryResponse, QueryVariable};
use sqlx::PgPool;
use starter_spi::Error;

use super::bind::{self, BindCtx, HostTokens, ScalarValue, TimeRange, VarValue};
use super::run::run_bound_query;
use super::QueryGuards;

/// Host-bound identity for a query, taken from the authenticated `Principal`.
/// The caller can never supply these — they feed `$caller_tenant_id` /
/// `$caller_user_id`.
#[derive(Debug, Clone, Default)]
pub struct QueryIdentity {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
}

/// Bind `req` against its macro context and `identity`, then execute it. The
/// single entry point a query handler calls: it never builds a `BindCtx` or
/// touches the runner itself.
pub async fn run_request(
    pool: &PgPool,
    req: &QueryRequest,
    identity: &QueryIdentity,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let ctx = to_bind_ctx(req, identity);
    let bound = bind::bind(&req.sql, &ctx)?;
    run_bound_query(pool, &bound, guards).await
}

/// Run a query-kind (WS-10): the kind's `sql` with its already-validated,
/// already-lowered named `params`, plus the same time/variable context and
/// host-bound `identity` as a raw query. The params reach the binder as
/// `$param` references and bind as `$N` args exactly like variables — so a kind
/// runs through the one shared binder, not a second engine. The kind registry
/// (in nexus-api) owns validation; this owns binding and execution.
pub async fn run_kind_request(
    pool: &PgPool,
    sql: &str,
    params: BTreeMap<String, super::bind::ParamValue>,
    req: &QueryRequest,
    identity: &QueryIdentity,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let mut ctx = to_bind_ctx(req, identity);
    ctx.params = params;
    let bound = bind::bind(sql, &ctx)?;
    run_bound_query(pool, &bound, guards).await
}

/// Build the binder context from the wire request and the caller's identity.
fn to_bind_ctx(req: &QueryRequest, identity: &QueryIdentity) -> BindCtx {
    BindCtx {
        time_range: req.time_range.map(|r| TimeRange {
            from: r.from,
            to: r.to,
        }),
        interval: req.interval_secs.map(Duration::from_secs),
        variables: req.variables.iter().map(to_var).collect(),
        params: BTreeMap::new(),
        host_tokens: HostTokens {
            caller_tenant_id: identity.tenant_id.clone(),
            caller_user_id: identity.user_id.clone(),
        },
    }
}

/// Lower a wire variable (a name + string values) into the binder's [`VarValue`].
/// A single value becomes `Single`; multiple values become `Multi` for list
/// expansion. Values stay strings on the wire — the binder binds them, so their
/// SQL type is the column's, decided by Postgres, not guessed here.
fn to_var(v: &QueryVariable) -> (String, VarValue) {
    let value = match v.values.as_slice() {
        [one] => VarValue::Single(ScalarValue::Text(one.clone())),
        many => VarValue::Multi(many.iter().cloned().map(ScalarValue::Text).collect()),
    };
    (v.name.clone(), value)
}
