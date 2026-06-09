//! Alert-rule persistence and the per-rule state-machine memory.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewRule, RulePatch, RuleRecord, RuleState};
use crate::tenant_tx;

const RULE_COLS: &str = "id, tenant_id, name, datasource_id, query, op, threshold, \
     for_secs, interval_secs, enabled, channel_ids, conditions, combinator, \
     no_data_policy, exec_error_policy, message_template";

/// Insert a rule. A duplicate name in the tenant is a `Conflict`.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewRule) -> Result<RuleRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_alert_rules \
         (tenant_id, name, datasource_id, query, op, threshold, for_secs, interval_secs, enabled, channel_ids, \
          conditions, combinator, no_data_policy, exec_error_policy, message_template) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(new.datasource_id)
    .bind(&new.query)
    .bind(&new.op)
    .bind(new.threshold)
    .bind(new.for_secs)
    .bind(new.interval_secs)
    .bind(new.enabled)
    .bind(&new.channel_ids)
    .bind(&new.conditions)
    .bind(&new.combinator)
    .bind(&new.no_data_policy)
    .bind(&new.exec_error_policy)
    .bind(&new.message_template)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    let id = row.get::<Uuid, _>("id");

    // The state row is created alongside the rule so the evaluator always finds
    // one; a rule with no state would be an evaluator special case for no reason.
    sqlx::query(
        "INSERT INTO nexus_alert_rule_state (rule_id, tenant_id, state) VALUES ($1, $2, 'ok')",
    )
    .bind(id)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(RuleRecord {
        id,
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        datasource_id: new.datasource_id,
        query: new.query.clone(),
        op: new.op.clone(),
        threshold: new.threshold,
        for_secs: new.for_secs,
        interval_secs: new.interval_secs,
        enabled: new.enabled,
        channel_ids: new.channel_ids.clone(),
        conditions: new.conditions.clone(),
        combinator: new.combinator.clone(),
        no_data_policy: new.no_data_policy.clone(),
        exec_error_policy: new.exec_error_policy.clone(),
        message_template: new.message_template.clone(),
    })
}

/// List the tenant's rules, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<RuleRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(&format!(
        "SELECT {RULE_COLS} FROM nexus_alert_rules ORDER BY created_at DESC"
    ))
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_rule).collect())
}

/// Fetch one rule by id within the tenant.
pub async fn get(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<Option<RuleRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(&format!(
        "SELECT {RULE_COLS} FROM nexus_alert_rules WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_rule))
}

/// Apply `patch` to rule `id`. Returns whether a row matched.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &RulePatch,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query(
        "UPDATE nexus_alert_rules SET \
           name              = COALESCE($2, name), \
           query             = COALESCE($3, query), \
           op                = COALESCE($4, op), \
           threshold         = COALESCE($5, threshold), \
           for_secs          = COALESCE($6, for_secs), \
           interval_secs     = COALESCE($7, interval_secs), \
           enabled           = COALESCE($8, enabled), \
           channel_ids       = COALESCE($9, channel_ids), \
           conditions        = COALESCE($10, conditions), \
           combinator        = COALESCE($11, combinator), \
           no_data_policy    = COALESCE($12, no_data_policy), \
           exec_error_policy = COALESCE($13, exec_error_policy), \
           message_template  = COALESCE($14, message_template) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.query)
    .bind(&patch.op)
    .bind(patch.threshold)
    .bind(patch.for_secs)
    .bind(patch.interval_secs)
    .bind(patch.enabled)
    .bind(patch.channel_ids.as_deref())
    .bind(&patch.conditions)
    .bind(&patch.combinator)
    .bind(&patch.no_data_policy)
    .bind(&patch.exec_error_policy)
    .bind(&patch.message_template)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Delete a rule (state/events/silences cascade). Returns whether a row matched.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_alert_rules WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Read a rule's state-machine memory.
pub async fn get_state(
    pool: &PgPool,
    tenant_id: &str,
    rule_id: Uuid,
) -> Result<Option<RuleState>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT rule_id, state, since, last_eval_at, last_value \
         FROM nexus_alert_rule_state WHERE rule_id = $1",
    )
    .bind(rule_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.map(|r| RuleState {
        rule_id: r.get::<Uuid, _>("rule_id"),
        state: r.get::<String, _>("state"),
        since: r.get::<DateTime<Utc>, _>("since"),
        last_eval_at: r.get::<Option<DateTime<Utc>>, _>("last_eval_at"),
        last_value: r.get::<Option<f64>, _>("last_value"),
    }))
}

/// Persist a rule's new state, stamping `last_eval_at` to now and the evaluated
/// value. `since` only advances when the state actually changed, so the
/// `for_secs` dwell measures time in the current state.
pub async fn put_state(
    pool: &PgPool,
    tenant_id: &str,
    rule_id: Uuid,
    state: &str,
    state_changed: bool,
    value: Option<f64>,
) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "UPDATE nexus_alert_rule_state SET \
           state = $2, \
           since = CASE WHEN $3 THEN now() ELSE since END, \
           last_eval_at = now(), \
           last_value = $4 \
         WHERE rule_id = $1",
    )
    .bind(rule_id)
    .bind(state)
    .bind(state_changed)
    .bind(value)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(())
}

fn row_to_rule(row: &sqlx::postgres::PgRow) -> RuleRecord {
    RuleRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        datasource_id: row.get::<Option<Uuid>, _>("datasource_id"),
        query: row.get::<String, _>("query"),
        op: row.get::<String, _>("op"),
        threshold: row.get::<f64, _>("threshold"),
        for_secs: row.get::<i32, _>("for_secs"),
        interval_secs: row.get::<i32, _>("interval_secs"),
        enabled: row.get::<bool, _>("enabled"),
        channel_ids: row.get::<Vec<Uuid>, _>("channel_ids"),
        conditions: row.get::<Option<serde_json::Value>, _>("conditions"),
        combinator: row.get::<String, _>("combinator"),
        no_data_policy: row.get::<String, _>("no_data_policy"),
        exec_error_policy: row.get::<String, _>("exec_error_policy"),
        message_template: row.get::<Option<String>, _>("message_template"),
    }
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "an alert rule with that name already exists".into(),
            };
        }
    }
    internal(e)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
