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
use super::condition::{self, Combinator, Condition, ConditionOutcome};
use super::notify::{self, Notification};
use super::policy::{self, Policy};
use super::reduce;
use super::template::{self, TemplateContext};
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

    // Run each of the rule's conditions under the same guards panels use, against
    // the rule's own datasource. Each condition reduces its result set to one
    // value and compares it; the combinator and the no-data/error policy then
    // resolve the single `breaching` boolean the pure state machine consumes — the
    // machine itself stays untouched. A legacy single-condition rule (no explicit
    // `conditions`) is the one-element case, so it behaves exactly as before.
    let datasource = resolve_pool(ctx, tenant, &rule).await?;
    let conditions = conditions_of(&rule);
    let prior_firing = state == State::Firing;
    let resolution = resolve_breaching(&datasource, &rule, &conditions, guards, prior_firing).await;
    let breaching = resolution.breaching;
    let value = resolution.value;

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

/// The conditions a rule evaluates. If the rule carries an explicit `conditions`
/// array it is used; otherwise the legacy single condition is reconstructed from
/// the top-level `query`/`op`/`threshold` (reducer `last`, matching the historical
/// "first row, first column" behaviour) so old rules evaluate unchanged.
fn conditions_of(rule: &RuleRecord) -> Vec<Condition> {
    if let Some(raw) = &rule.conditions {
        if let Ok(parsed) = serde_json::from_value::<Vec<Condition>>(raw.clone()) {
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    vec![Condition {
        query: rule.query.clone(),
        reducer: "last".to_string(),
        op: rule.op.clone(),
        threshold: rule.threshold,
    }]
}

/// The combined result of evaluating every condition of a rule: the breaching
/// boolean (after combinator + no-data/error policy) and a representative value
/// for the event record and the notification template (the first condition's
/// reduced value).
struct Resolution {
    breaching: bool,
    value: Option<f64>,
}

/// Evaluate every condition, combine them, and apply the no-data/error policies.
/// A query error routes through `exec_error_policy`; an empty result through
/// `no_data_policy`. Both stay pure given `prior_firing` (the input to
/// `keep_last`), so the state machine never sees policy logic.
async fn resolve_breaching(
    datasource: &PgPool,
    rule: &RuleRecord,
    conditions: &[Condition],
    guards: nexus_store::QueryGuards,
    prior_firing: bool,
) -> Resolution {
    let mut outcomes = Vec::with_capacity(conditions.len());
    let mut first_value = None;
    for cond in conditions {
        match evaluate_condition(datasource, cond, guards).await {
            Ok(outcome) => {
                if first_value.is_none() {
                    first_value = outcome.value;
                }
                outcomes.push(outcome);
            }
            Err(e) => {
                // A query that errored cannot contribute a value; the error
                // policy decides the whole rule's breaching, short-circuiting.
                tracing::warn!(rule_id = %rule.id, error = %e, "alert condition query failed");
                let breaching =
                    policy::resolve(Policy::parse(&rule.exec_error_policy), prior_firing);
                return Resolution {
                    breaching,
                    value: first_value,
                };
            }
        }
    }

    // A missing input makes the combined result undefined → the no-data policy.
    if condition::any_no_data(&outcomes) {
        let breaching = policy::resolve(Policy::parse(&rule.no_data_policy), prior_firing);
        return Resolution {
            breaching,
            value: first_value,
        };
    }

    let breaching = condition::combine(&outcomes, Combinator::parse(&rule.combinator));
    Resolution {
        breaching,
        value: first_value,
    }
}

/// Evaluate one condition: run its query, reduce the rows to a value, and compare.
/// An empty result is the no-data case (`had_data == false`, non-breaching here —
/// the rule-level policy decides what that means for the whole rule).
async fn evaluate_condition(
    datasource: &PgPool,
    cond: &Condition,
    guards: nexus_store::QueryGuards,
) -> Result<ConditionOutcome, String> {
    let resp = nexus_store::run_query(datasource, &cond.query, guards)
        .await
        .map_err(stringify)?;
    let reduced = reduce::reduce(&resp.rows, condition::reducer_of(cond));
    match reduced {
        Some(v) => {
            let breaching = breaches(v, &cond.op, cond.threshold)?;
            Ok(ConditionOutcome {
                breaching,
                had_data: true,
                value: Some(v),
            })
        }
        None => Ok(ConditionOutcome {
            breaching: false,
            had_data: false,
            value: None,
        }),
    }
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

/// Write the transition event, and — unless an active silence covers the rule —
/// deliver it to each channel with retry/backoff, recording the per-channel
/// outcome (delivered, or failed after N attempts) on the event.
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
        let message = render_message(rule, transition, value);
        let notification = Notification {
            rule_name: rule.name.clone(),
            transition: transition.as_str().to_string(),
            value,
            threshold: rule.threshold,
            op: rule.op.clone(),
            message,
        };
        let channels = channel::by_ids(metadata, tenant, &rule.channel_ids)
            .await
            .map_err(stringify)?;
        let mut notes = Vec::new();
        for ch in &channels {
            let outcome = notify::deliver_with_retry(ch, &notification).await;
            if outcome.succeeded() {
                notified = true;
                if outcome.attempts > 1 {
                    notes.push(format!("{}: delivered after {} attempts", ch.name, outcome.attempts));
                }
            } else if let Some(err) = outcome.last_error {
                notes.push(format!("{}: failed after {} attempts ({err})", ch.name, outcome.attempts));
            }
        }
        if !notes.is_empty() {
            detail = Some(notes.join("; "));
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

/// Render the notification text from the rule's template (or the default),
/// safely substituting the fixed token set with this transition's values.
fn render_message(rule: &RuleRecord, transition: Transition, value: Option<f64>) -> String {
    let template = rule
        .message_template
        .as_deref()
        .unwrap_or(template::DEFAULT_TEMPLATE);
    template::render(
        template,
        &TemplateContext {
            rule_name: &rule.name,
            state: transition.as_str(),
            op: &rule.op,
            threshold: rule.threshold,
            value,
        },
    )
}

fn stringify(e: impl std::fmt::Display) -> String {
    e.to_string()
}
