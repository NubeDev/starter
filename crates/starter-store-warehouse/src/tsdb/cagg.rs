//! Continuous-aggregate execution helpers.
//!
//! The DDL strings themselves are produced by
//! [`starter_warehouse::ddl::TimescaleDbDialect`]; this module
//! drives them against a [`WarehouseClient`] pool and exposes the
//! snapshot read path (proposal §"The mart / continuous aggregate
//! translation").

use sqlx::Row;

use super::client::{WarehouseClient, WarehouseError};

/// Snapshot row returned by [`view_snapshot`]. Mirrors the
/// fields the proposal calls out: the rendered `SELECT` from
/// `pg_get_viewdef` is the "DDL" we capture for the reversible.
#[derive(Debug, Clone)]
pub struct CaggSnapshot {
    pub view_name: String,
    pub view_definition: String,
    pub materialization_hypertable_name: String,
}

/// Read the recorded definition of a continuous aggregate.
/// Joins `timescaledb_information.continuous_aggregates` with
/// `pg_get_viewdef` for the view oid — exactly the snapshot the
/// rule verb's `WarehouseRuleReversible` records.
pub async fn view_snapshot(
    client: &WarehouseClient,
    view_name: &str,
) -> Result<Option<CaggSnapshot>, WarehouseError> {
    let row = sqlx::query(
        "SELECT \
           ca.view_name, \
           pg_get_viewdef(format('%I.%I', ca.view_schema, ca.view_name)::regclass) \
             AS view_definition, \
           ca.materialization_hypertable_name \
         FROM timescaledb_information.continuous_aggregates AS ca \
         WHERE ca.view_name = $1",
    )
    .bind(view_name)
    .fetch_optional(client.pool())
    .await?;
    Ok(row.map(|r| CaggSnapshot {
        view_name: r.try_get("view_name").unwrap_or_default(),
        view_definition: r.try_get("view_definition").unwrap_or_default(),
        materialization_hypertable_name: r
            .try_get("materialization_hypertable_name")
            .unwrap_or_default(),
    }))
}

/// Force a refresh of a continuous aggregate over the given
/// window. Used by the smoke test to materialise inserted rows
/// before asserting on the cagg.
pub async fn refresh(
    client: &WarehouseClient,
    view_name: &str,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<(), WarehouseError> {
    // `CALL refresh_continuous_aggregate($1, $2, $3)` cannot bind
    // the view name (it's an identifier, not a value). We use a
    // format string with a quoted identifier; the caller is
    // expected to pass a validated name from the DDL dialect.
    let stmt = format!("CALL refresh_continuous_aggregate('{view_name}', $1, $2)");
    sqlx::query(&stmt)
        .bind(start)
        .bind(end)
        .execute(client.pool())
        .await?;
    Ok(())
}
