//! Retention policy helpers, port of the `WarehouseRetention…`
//! verbs to TimescaleDB.
//!
//! The `WarehouseRetentionReversible` snapshot shape — `{
//! table_name, days }` — is unchanged from the ClickHouse path
//! (proposal §"Retention"). Only the SQL changes.

use sqlx::Row;

use super::client::{WarehouseClient, WarehouseError};

/// Add (or overwrite) a retention policy on a hypertable.
/// Implements `rubix.warehouse.retention.set`. The caller is
/// responsible for having validated `table` through the same
/// identifier guard the DDL dialect uses — this helper does NOT
/// re-validate so it can be called from contexts where the
/// identifier is already trusted.
pub async fn add_retention_policy(
    client: &WarehouseClient,
    table: &str,
    days: i32,
) -> Result<(), WarehouseError> {
    // remove first to make the operation idempotent w.r.t. the
    // previous value — `add_retention_policy` errors if a policy
    // already exists.
    let _ = remove_retention_policy(client, table).await;
    let stmt = format!(
        "SELECT add_retention_policy('{table}', INTERVAL '{days} days', if_not_exists => TRUE)",
    );
    sqlx::query(&stmt).execute(client.pool()).await?;
    Ok(())
}

/// Remove the retention policy on a hypertable. Idempotent in
/// the Timescale sense: `if_exists => TRUE` makes a missing
/// policy a no-op rather than an error.
pub async fn remove_retention_policy(
    client: &WarehouseClient,
    table: &str,
) -> Result<(), WarehouseError> {
    let stmt = format!("SELECT remove_retention_policy('{table}', if_exists => TRUE)");
    sqlx::query(&stmt).execute(client.pool()).await?;
    Ok(())
}

/// Snapshot the current retention setting for a hypertable.
/// Returns the configured `drop_after` interval in days, or
/// `None` if no policy is set. Implements the snapshot path the
/// proposal calls out:
///
/// ```sql
/// SELECT config FROM timescaledb_information.jobs
/// WHERE proc_name = 'policy_retention' AND hypertable_name = $1;
/// ```
pub async fn snapshot_days(
    client: &WarehouseClient,
    table: &str,
) -> Result<Option<i32>, WarehouseError> {
    let row = sqlx::query(
        "SELECT config FROM timescaledb_information.jobs \
         WHERE proc_name = 'policy_retention' AND hypertable_name = $1",
    )
    .bind(table)
    .fetch_optional(client.pool())
    .await?;
    let Some(row) = row else { return Ok(None) };
    let config: serde_json::Value = row.try_get("config")?;
    // `config -> 'drop_after'` is an ISO-8601 interval string
    // ("P30D" or "30 days" depending on Timescale version); we
    // accept either by parsing for a leading integer.
    let raw = config
        .get("drop_after")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let days = parse_days(raw);
    Ok(days)
}

fn parse_days(raw: &str) -> Option<i32> {
    // Accept "P30D" (ISO-8601) and "30 days" / "30 day" (Timescale
    // textual form). Return the integer day count.
    if let Some(rest) = raw.strip_prefix('P') {
        if let Some(num) = rest.strip_suffix('D') {
            return num.parse().ok();
        }
    }
    let trimmed = raw.trim();
    let mut split = trimmed.split_whitespace();
    let num = split.next()?;
    let unit = split.next().unwrap_or("days");
    if unit.starts_with("day") {
        num.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::parse_days;

    #[test]
    fn iso_form() {
        assert_eq!(parse_days("P30D"), Some(30));
    }

    #[test]
    fn textual_form() {
        assert_eq!(parse_days("30 days"), Some(30));
        assert_eq!(parse_days("1 day"), Some(1));
    }

    #[test]
    fn rejects_non_day_intervals() {
        assert_eq!(parse_days("12 hours"), None);
    }
}
