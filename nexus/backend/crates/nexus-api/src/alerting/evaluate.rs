//! Evaluate one alert rule: query → compare → step the state machine → persist
//! → on a transition, record an event and (unless silenced) notify.
//!
//! This is the orchestration the design calls the evaluator. It is deliberately
//! the only place that ties the store, the guarded query path, the pure state
//! machine, and the notifiers together — each of those stays independently
//! testable, and this composes them for one rule under its tenant's RLS context.

use chrono::Utc;
use nexus_store::datasource::Envelope;
use nexus_store::alert::{channel, event, rule, silence, NewEvent, RuleRecord};
use sqlx::PgPool;
use uuid::Uuid;

use super::compare::breaches;
use super::notify::{self, Notification};
use super::transition::{step, State, Transition};
use crate::datasource_pools::DatasourcePools;

/// The handles one rule evaluation needs to reach its datasource: the
/// control-plane pool (where the datasource records live), the secret envelope,
/// the per-datasource pool cache, and the dev fallback pool for rules that carry
/// no datasource id. Bundled so the signature stays readable as the evaluator
/// resolves the rule's *own* datasource rather than querying a single shared one.
pub struct EvalContext<'a> {
    pub metadata: &'a PgPool,
    pub envelope: &'a Envelope,
    pub pools: &'a DatasourcePools,
    pub dev_pool: &'a PgPool,
    pub guards: nexus_store::QueryGuards,
}

/// Evaluate the rule `id` for `tenant`. The rule's query runs against the
/// datasource the rule names (resolved through the cache, under the standard
/// guards); a rule with no datasource falls back to the dev pool. Errors are
/// logged and swallowed per rule so one bad rule never stalls the scheduler.
pub async fn evaluate_rule(ctx: &EvalContext<'_>, tenant: &str, id: Uuid) {
    if let Err(e) = try_evaluate(ctx, tenant, id).await {
        tracing::warn!(tenant, rule_id = %id, error = %e, "alert evaluation failed");
    }
}

async fn try_evaluate(ctx: &EvalContext<'_>, tenant: &str, id: Uuid) -> Result<(), String> {
    let metadata = ctx.metadata;
    let guards = ctx.guards;
    let Some(rule) = rule::get(metadata, tenant, id).await.map_err(stringify)? else {
        return Ok(()); // deleted between claim and evaluate — nothing to do
    };
    let state = rule::get_state(metadata, tenant, id)
        .await
        .map_err(stringify)?
        .map(|s| State::parse(&s.state))
        .unwrap_or(State::Ok);
    let since = rule::get_state(metadata, tenant, id)
        .await
        .map_err(stringify)?
        .map(|s| s.since);

    // Run the rule's query under the same guards panels use, against the rule's
    // own datasource, and read the first numeric cell. No row is "no data" — a
    // non-breaching reading that must not flap the rule to firing.
    let datasource = resolve_pool(ctx, tenant, &rule).await?;
    let value = evaluate_value(&datasource, &rule.query, guards).await?;
    let breaching = match value {
        Some(v) => breaches(v, &rule.op, rule.threshold)?,
        None => false,
    };

    // The dwell has elapsed when the rule has been pending at least `for_secs`.
    let dwell_elapsed = match (state, since) {
        (State::Pending, Some(since)) => {
            (Utc::now() - since).num_seconds() >= rule.for_secs as i64
        }
        // From ok/resolved a zero dwell fires at once; a non-zero dwell routes
        // through pending first (handled by the machine).
        _ => rule.for_secs == 0,
    };

    let outcome = step(state, breaching, dwell_elapsed);
    rule::put_state(
        metadata,
        tenant,
        id,
        outcome.next.as_str(),
        outcome.changed,
        value,
    )
    .await
    .map_err(stringify)?;

    if let Some(transition) = outcome.transition {
        record_and_notify(metadata, tenant, &rule, transition, value).await?;
    }
    Ok(())
}

/// The pool the rule's query runs against: its named datasource (resolved under
/// the tenant's RLS and cached after first build), or the dev pool when the rule
/// carries no datasource id. The audit actor is the rule id — the decrypt log
/// then points at which rule triggered the connection.
async fn resolve_pool(
    ctx: &EvalContext<'_>,
    tenant: &str,
    rule: &RuleRecord,
) -> Result<PgPool, String> {
    let Some(ds_id) = rule.datasource_id else {
        return Ok(ctx.dev_pool.clone());
    };
    let record = nexus_store::datasource::get(ctx.metadata, tenant, ds_id)
        .await
        .map_err(stringify)?
        .ok_or_else(|| format!("rule names datasource {ds_id}, not visible to {tenant}"))?;
    ctx.pools
        .get_or_connect(ctx.metadata, ctx.envelope, tenant, &rule.id.to_string(), &record)
        .await
        .map_err(stringify)
}

/// Run the query and pull the first row's first column as f64. Returns `None`
/// when the query yields no rows (no data).
async fn evaluate_value(
    datasource: &PgPool,
    query: &str,
    guards: nexus_store::QueryGuards,
) -> Result<Option<f64>, String> {
    let resp = nexus_store::run_query(datasource, query, guards)
        .await
        .map_err(stringify)?;
    let Some(first) = resp.rows.first() else {
        return Ok(None);
    };
    let obj = first
        .as_object()
        .ok_or_else(|| "alert query row is not an object".to_string())?;
    let cell = obj
        .values()
        .next()
        .ok_or_else(|| "alert query returned no columns".to_string())?;
    cell.as_f64()
        .ok_or_else(|| "alert query first column is not numeric".to_string())
        .map(Some)
}

/// Write the transition event, and — unless an active silence covers the rule —
/// deliver it to each channel, recording whether delivery succeeded.
async fn record_and_notify(
    metadata: &PgPool,
    tenant: &str,
    rule: &RuleRecord,
    transition: Transition,
    value: Option<f64>,
) -> Result<(), String> {
    let silenced = silence::is_silenced(metadata, tenant, rule.id, Utc::now())
        .await
        .map_err(stringify)?;

    let mut notified = false;
    let mut detail: Option<String> = None;
    if !silenced {
        let notification = Notification {
            rule_name: rule.name.clone(),
            transition: transition.as_str().to_string(),
            value,
            threshold: rule.threshold,
            op: rule.op.clone(),
        };
        let channels = channel::by_ids(metadata, tenant, &rule.channel_ids)
            .await
            .map_err(stringify)?;
        let mut failures = Vec::new();
        for ch in &channels {
            match notify::deliver(ch, &notification).await {
                Ok(()) => notified = true,
                Err(e) => failures.push(format!("{}: {e}", ch.name)),
            }
        }
        if !failures.is_empty() {
            detail = Some(failures.join("; "));
        }
    }

    event::insert(
        metadata,
        tenant,
        &NewEvent {
            rule_id: rule.id,
            transition: transition.as_str().to_string(),
            value,
            silenced,
            notified,
            detail,
        },
    )
    .await
    .map_err(stringify)?;
    Ok(())
}

fn stringify(e: impl std::fmt::Display) -> String {
    e.to_string()
}
