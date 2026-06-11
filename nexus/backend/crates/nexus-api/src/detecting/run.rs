//! Run one detection: query → insight-over-frame → upsert/resolve findings.
//!
//! This is the analytic sibling of [`crate::alerting::evaluate`]. Where the
//! alert evaluator reduces a query to one scalar and compares it, the detection
//! runner runs the tenant's stored insight (RW-06) over the *whole* query frame
//! and records one finding per flagged row. It deliberately reuses the same
//! guarded query path and datasource resolution, and the insight sandbox, so a
//! detection can do no more than a panel query + a preview already can — it just
//! does it on a schedule and writes findings.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use nexus_store::datasource::Envelope;
use nexus_store::detection::{self, DetectionRecord};
use nexus_store::finding::{self, NewFinding};
use serde_json::{Map, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::datasource_pools::DatasourcePools;
use crate::state::AppState;

/// The handles one detection run needs to reach its datasource and stores.
/// Mirrors [`crate::alerting::evaluate::EvalContext`]. `state` is carried so a
/// detection that names federated `sources` can dispatch through the same
/// federation engine + result cache a panel query uses; the single-datasource
/// path uses only the individual handles, exactly like the alert evaluator.
pub struct RunContext<'a> {
    pub state: &'a AppState,
    pub metadata: &'a PgPool,
    pub envelope: &'a Envelope,
    pub pools: &'a DatasourcePools,
    pub dev_pool: &'a PgPool,
    pub guards: nexus_store::QueryGuards,
}

/// Run detection `id` for `tenant`. Errors are logged and swallowed per
/// detection so one bad detection never stalls the scheduler.
pub async fn run_detection(ctx: &RunContext<'_>, tenant: &str, id: Uuid) {
    if let Err(e) = try_run(ctx, tenant, id).await {
        tracing::warn!(tenant, detection_id = %id, error = %e, "detection run failed");
    }
}

async fn try_run(ctx: &RunContext<'_>, tenant: &str, id: Uuid) -> Result<(), String> {
    let Some(det) = detection::get(ctx.metadata, tenant, id)
        .await
        .map_err(stringify)?
    else {
        return Ok(()); // deleted between claim and run — nothing to do
    };

    // Resolve the insight script under the tenant. RESTRICT on the FK means a
    // detection always has a live insight; a race that deletes it leaves the
    // detection without a rule, which we treat as a logged no-op.
    let Some(insight) = nexus_store::insight::by_id(ctx.metadata, tenant, det.insight_id)
        .await
        .map_err(stringify)?
    else {
        return Err(format!("detection {id} references missing insight {}", det.insight_id));
    };

    // Run the query under the same guards panels use. A detection that names
    // federated `sources` dispatches through the federation engine (cross-
    // datasource / file joins); otherwise it's a single-datasource push-down
    // against the detection's own datasource (or the dev pool when it names none).
    let response = run_detection_query(ctx, tenant, &det).await?;

    // Run the insight over the result frame. The engine never grows the row
    // count, so the query cap already bounds how many findings a run can emit.
    let params = if det.params.is_null() {
        Value::Object(Map::new())
    } else {
        det.params.clone()
    };
    let rows = nexus_insights::run_insight_rows(insight.script, response.rows, params)
        .await
        .map_err(stringify)?;

    // Turn each flagged row into a finding payload, then reconcile in one tenant
    // transaction (upsert flagged, auto-resolve the rest).
    let flagged = flagged_findings(&det, &rows);
    let (upserts, resolved) = finding::reconcile(ctx.metadata, tenant, det.id, &flagged)
        .await
        .map_err(stringify)?;
    tracing::debug!(
        tenant, detection_id = %id, flagged = flagged.len(), upserts, resolved,
        "detection run complete"
    );
    Ok(())
}

/// Run the detection's query and return its rows. Two paths, mirroring the query
/// route's dispatch: federated when `sources` is non-empty (resolved + run
/// through the RW-05 engine under a tenant-system principal), single-datasource
/// push-down otherwise. Both honour the same `QueryGuards`.
async fn run_detection_query(
    ctx: &RunContext<'_>,
    tenant: &str,
    det: &DetectionRecord,
) -> Result<nexus_spi::dto::query::QueryResponse, String> {
    let sources = parse_sources(&det.sources)?;
    if sources.is_empty() {
        let datasource = resolve_pool(ctx, tenant, det).await?;
        return nexus_store::run_query(&datasource, &det.sql, ctx.guards)
            .await
            .map_err(stringify);
    }

    // A scheduled detection is a tenant-system actor. The federation resolver
    // checks `view` on each named datasource, so the run executes as a tenant
    // admin — the same authority the alert scheduler reads any tenant datasource
    // with (its `resolve_pool` is RLS-scoped to the tenant). The audit actor on
    // each secret-decrypt is the detection id (via `principal.subject`).
    let principal = detection_principal(tenant, det.id);
    let req = nexus_spi::dto::query::QueryRequest {
        sql: det.sql.clone(),
        time_range: None,
        interval_secs: None,
        variables: Vec::new(),
        kind: None,
        params: None,
        sources,
        insight: None,
    };
    let identity = nexus_store::QueryIdentity {
        tenant_id: Some(tenant.to_string()),
        user_id: Some(principal.subject.clone()),
    };
    crate::federation::run_cached(ctx.state, &principal, tenant, &req, &identity)
        .await
        .map_err(stringify)
}

/// Parse the stored `sources` jsonb into the typed federated-source refs. A
/// malformed array is a detection-config error, surfaced (not silently ignored)
/// so a broken detection is visible in the run log rather than quietly running
/// the bare `sql` against the wrong place.
fn parse_sources(
    raw: &Value,
) -> Result<Vec<nexus_spi::dto::query::FederatedSourceRef>, String> {
    if raw.is_null() {
        return Ok(Vec::new());
    }
    serde_json::from_value(raw.clone())
        .map_err(|e| format!("detection sources are not a valid source list: {e}"))
}

/// A tenant-system principal for the detection's federated run: admin role so
/// the per-source `view` check passes, subject = the detection id so the
/// decrypt audit points at which detection opened the connection.
fn detection_principal(tenant: &str, detection_id: Uuid) -> starter_spi::auth::Principal {
    starter_spi::auth::Principal {
        subject: detection_id.to_string(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: Some(tenant.to_string()),
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

/// Build the finding payloads for the rows the insight flagged. A row is flagged
/// when its `flag_column` is truthy. The `target` is the detection's
/// `target_columns` projected from the row; `value` comes from `value_column`;
/// `context` is every other column (the "why"); `dedup_key` is the stable hash
/// of the target values so a re-flag updates one finding instead of spawning N.
fn flagged_findings(det: &DetectionRecord, rows: &[Value]) -> Vec<NewFinding> {
    let mut out = Vec::new();
    // An empty `flag_column` means the insight already filtered: every returned
    // row is a finding (the WS "find high usage" = `df.filter_gt(...)` pattern,
    // where the shrunk frame carries no boolean flag). A named column gates on
    // its truthiness (the `anomalies(...)` → `value_anomaly` pattern).
    let flag_all = det.flag_column.is_empty();
    for row in rows {
        let Some(obj) = row.as_object() else { continue };
        if !flag_all && !is_truthy(obj.get(&det.flag_column)) {
            continue;
        }

        let mut target = Map::new();
        for col in &det.target_columns {
            if let Some(v) = obj.get(col) {
                target.insert(col.clone(), v.clone());
            }
        }

        let value = det
            .value_column
            .as_ref()
            .and_then(|c| obj.get(c))
            .and_then(Value::as_f64);

        // Context = the row minus the flag and target columns: the derived
        // evidence an analyst reads to understand the spark.
        let mut context = Map::new();
        for (k, v) in obj {
            if k == &det.flag_column || det.target_columns.contains(k) {
                continue;
            }
            context.insert(k.clone(), v.clone());
        }

        let at = extract_time(obj).unwrap_or_else(Utc::now);

        out.push(NewFinding {
            detection_id: det.id,
            at,
            dedup_key: dedup_key(det.id, &target),
            target: Value::Object(target),
            value,
            context: Value::Object(context),
        });
    }
    out
}

/// Truthy test for the flag column: a JSON `true`, a non-zero number, or a
/// non-empty/"true" string all count. The canonical insight primitive
/// (`anomalies`) emits a real boolean, so the common path is `Bool(true)`.
fn is_truthy(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty() && !s.eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// A stable dedup key from the detection id and the target column values. The
/// target map's key order is deterministic (BTreeMap-backed serde object), so
/// the same target hashes identically across runs. Target-only granularity (the
/// WS "lean"): one open finding per target until it resolves; its history is the
/// audit/ack trail, time-series lives in trends.
fn dedup_key(detection_id: Uuid, target: &Map<String, Value>) -> String {
    let mut hasher = DefaultHasher::new();
    detection_id.hash(&mut hasher);
    // serde_json::Map iterates in sorted key order by default, so this is stable
    // regardless of the source row's column order.
    for (k, v) in target {
        k.hash(&mut hasher);
        v.to_string().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Pull an event time off the row if it carries a recognisable timestamp column
/// (`time`/`ts`/`at`/`timestamp`), parsed as RFC 3339. Absent or unparseable ⇒
/// the caller falls back to now().
fn extract_time(obj: &Map<String, Value>) -> Option<DateTime<Utc>> {
    const TIME_KEYS: [&str; 4] = ["time", "ts", "at", "timestamp"];
    for key in TIME_KEYS {
        if let Some(Value::String(s)) = obj.get(key) {
            if let Ok(t) = DateTime::parse_from_rfc3339(s) {
                return Some(t.with_timezone(&Utc));
            }
        }
    }
    None
}

/// The pool the detection's query runs against: its named datasource (resolved
/// under the tenant's RLS and cached), or the dev pool when it names none. The
/// audit actor is the detection id. Mirrors the alert evaluator's `resolve_pool`.
async fn resolve_pool(
    ctx: &RunContext<'_>,
    tenant: &str,
    det: &DetectionRecord,
) -> Result<PgPool, String> {
    let Some(ds_id) = det.datasource_id else {
        return Ok(ctx.dev_pool.clone());
    };
    let record = nexus_store::datasource::get(ctx.metadata, tenant, ds_id)
        .await
        .map_err(stringify)?
        .ok_or_else(|| format!("detection names datasource {ds_id}, not visible to {tenant}"))?;
    ctx.pools
        .get_or_connect(ctx.metadata, ctx.envelope, tenant, &det.id.to_string(), &record)
        .await
        .map_err(stringify)
}

fn stringify(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn det() -> DetectionRecord {
        DetectionRecord {
            id: Uuid::nil(),
            tenant_id: "t".into(),
            name: "d".into(),
            insight_id: Uuid::nil(),
            datasource_id: None,
            sql: "select 1".into(),
            params: Value::Null,
            sources: Value::Array(Vec::new()),
            flag_column: "value_anomaly".into(),
            target_columns: vec!["site".into(), "meter".into()],
            value_column: Some("value".into()),
            for_secs: 0,
            interval_secs: 300,
            enabled: true,
        }
    }

    #[test]
    fn only_flagged_rows_become_findings() {
        let rows = vec![
            json!({"site":"s1","meter":"m1","value":9.0,"value_anomaly":true,"z":3.1}),
            json!({"site":"s1","meter":"m2","value":2.0,"value_anomaly":false,"z":0.1}),
        ];
        let f = flagged_findings(&det(), &rows);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].value, Some(9.0));
        assert_eq!(f[0].target, json!({"site":"s1","meter":"m1"}));
        // Context carries the evidence, minus flag + target columns.
        assert_eq!(f[0].context, json!({"value":9.0,"z":3.1}));
    }

    #[test]
    fn dedup_key_is_stable_and_target_distinct() {
        let d = det();
        let a = json!({"site":"s1","meter":"m1","value":1.0,"value_anomaly":true});
        let b = json!({"meter":"m1","site":"s1","value":2.0,"value_anomaly":true}); // reordered
        let c = json!({"site":"s1","meter":"m2","value":1.0,"value_anomaly":true});
        let fa = flagged_findings(&d, std::slice::from_ref(&a));
        let fb = flagged_findings(&d, std::slice::from_ref(&b));
        let fc = flagged_findings(&d, std::slice::from_ref(&c));
        assert_eq!(fa[0].dedup_key, fb[0].dedup_key, "key stable across column order");
        assert_ne!(fa[0].dedup_key, fc[0].dedup_key, "distinct target ⇒ distinct key");
    }

    #[test]
    fn empty_flag_column_flags_every_row() {
        let mut d = det();
        d.flag_column = String::new(); // the filter_gt pattern: frame already shrunk
        let rows = vec![
            json!({"site":"s1","meter":"m1","value":9.0}),
            json!({"site":"s1","meter":"m2","value":8.0}),
        ];
        let f = flagged_findings(&d, &rows);
        assert_eq!(f.len(), 2, "every returned row is a finding");
        assert_eq!(f[0].context, json!({"value":9.0}), "context excludes only target");
    }

    #[test]
    fn parse_sources_empty_and_populated() {
        // Null and an empty array both mean "single-datasource push-down".
        assert!(parse_sources(&Value::Null).unwrap().is_empty());
        assert!(parse_sources(&json!([])).unwrap().is_empty());
        // A well-formed source list parses into typed refs (the federation path).
        let parsed = parse_sources(&json!([
            { "alias": "pg", "datasource": "00000000-0000-0000-0000-000000000000", "table": "usage" }
        ]))
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].alias, "pg");
        assert_eq!(parsed[0].table.as_deref(), Some("usage"));
        // A malformed list is surfaced as an error, never silently dropped.
        assert!(parse_sources(&json!([{ "alias": 5 }])).is_err());
    }

    #[test]
    fn detection_principal_is_tenant_admin() {
        let p = detection_principal("acme", Uuid::nil());
        assert_eq!(p.tenant_id.as_deref(), Some("acme"));
        assert!(matches!(p.role, starter_spi::auth::Role::Admin));
        // Subject = detection id so the decrypt audit names the detection.
        assert_eq!(p.subject, Uuid::nil().to_string());
    }

    #[test]
    fn truthy_covers_bool_number_string() {
        assert!(is_truthy(Some(&json!(true))));
        assert!(is_truthy(Some(&json!(1))));
        assert!(is_truthy(Some(&json!("yes"))));
        assert!(!is_truthy(Some(&json!(false))));
        assert!(!is_truthy(Some(&json!(0))));
        assert!(!is_truthy(Some(&json!("false"))));
        assert!(!is_truthy(None));
    }
}
