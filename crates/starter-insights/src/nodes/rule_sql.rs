//! `starter.flow.rule.sql` — parameterised SELECT against the host
//! primary store (Insights SCOPE D2 Phase 1 shape).
//!
//! Phase 2 ships the host-primary-store variant only — no
//! `SqlSource` SPI yet. The node is gated on the `sqlite` cargo
//! feature because the only primary store this crate knows about
//! today is SQLite.
//!
//! Input slots:
//! - `sql` ([`SlotValue::String`], required) — a single SELECT.
//!   Multiple statements are rejected at the engine layer
//!   (sqlx::query refuses) and surface as a rule-error verdict.
//! - `params` ([`SlotValue::Json`], optional) — a JSON array of
//!   primitives bound positionally to `?` placeholders.
//! - `rule_id` ([`SlotValue::String`], optional) — explicit id; if
//!   absent, D4 anonymous id over the SQL body.
//!
//! Output slot:
//! - `dataset` ([`SlotValue::Json`]) — a JSON projection of the
//!   resulting [`Dataset`] (same shape as `window.*`). Coverage is
//!   `full_point` because SQL is a single-shot SELECT, not a
//!   sampled window; downstream nodes that need a richer coverage
//!   should wrap the result in `align`.

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value as Json;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    rule_error_flag, Coverage, RuleErrorKind, RuleId, Severity, Tags, TimeZoneId, Verdict,
};
use starter_store_sqlite::pool::Pool;

use crate::nodes::rule_rhai::anon_rule_id;
use super::VERDICT_SLOT;

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.rule.sql";

/// Required input slot — the SQL body.
pub const SQL_SLOT: &str = "sql";

/// Optional input slot — JSON array of bind params.
pub const PARAMS_SLOT: &str = "params";

/// Optional input slot — explicit RuleId.
pub const RULE_ID_SLOT: &str = "rule_id";

/// Output slot — JSON Dataset projection.
pub const DATASET_SLOT: &str = "dataset";

/// Body for `starter.flow.rule.sql`.
pub struct RuleSqlNode {
    kind: KindId,
    pool: Pool,
}

impl RuleSqlNode {
    /// Construct a SQL-rule body bound to the host's primary
    /// SQLite pool. D2 Phase 1 ships this single seam; attached
    /// read-only datasources land in Phase 2's follow-up via a
    /// `SqlSource` SPI.
    pub fn new(pool: Pool) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("kind id is valid"),
            pool,
        }
    }
}

#[async_trait]
impl NodeBehavior for RuleSqlNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let sql = match input.remove(SQL_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Ok(emit_error(
                    RuleId::new("starter.rule", "unknown", 1),
                    RuleErrorKind::InputMissing,
                    "rule.sql: missing `sql` input slot",
                ));
            }
        };
        let rule_id = match input.remove(RULE_ID_SLOT) {
            Some(SlotValue::String(s)) => {
                crate::nodes::rule_rust::parse_rule_id(&s).unwrap_or_else(|| anon_rule_id(&sql))
            }
            _ => anon_rule_id(&sql),
        };
        let params = match input.remove(PARAMS_SLOT) {
            None | Some(SlotValue::Null) => Vec::new(),
            Some(SlotValue::Json(Json::Array(a))) => a,
            other => {
                return Ok(emit_error(
                    rule_id,
                    RuleErrorKind::InputMissing,
                    format!("rule.sql: `params` must be a JSON array; got {other:?}"),
                ));
            }
        };

        // Build the query and bind primitives in order. Sqlx
        // refuses multi-statement strings, so SQL-injection via
        // semicolons is contained at the driver level.
        let mut q = sqlx::query(&sql);
        for p in &params {
            q = match p {
                Json::Null => q.bind(None::<String>),
                Json::Bool(b) => q.bind(*b),
                Json::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        q.bind(f)
                    } else {
                        q.bind(n.to_string())
                    }
                }
                Json::String(s) => q.bind(s.clone()),
                other => q.bind(other.to_string()),
            };
        }

        let rows = match q.fetch_all(self.pool.sqlx()).await {
            Ok(rs) => rs,
            Err(e) => {
                return Ok(emit_error(
                    rule_id,
                    RuleErrorKind::BodyFailed,
                    format!("rule.sql: query failed: {e}"),
                ));
            }
        };

        // Project rows to JSON objects keyed by column name. SQLite
        // only knows int/float/text/blob/null; we map each
        // accordingly.
        let mut json_rows: Vec<Json> = Vec::with_capacity(rows.len());
        let mut columns: Vec<String> = Vec::new();
        if let Some(first) = rows.first() {
            use sqlx::{Column, Row};
            columns = first
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
        }
        for r in &rows {
            use sqlx::{Column, Row, TypeInfo, ValueRef};
            let mut obj = serde_json::Map::new();
            for (i, col) in r.columns().iter().enumerate() {
                let raw = r.try_get_raw(i).ok();
                let value = raw
                    .map(|v| {
                        if v.is_null() {
                            Json::Null
                        } else {
                            let ty = v.type_info();
                            let tyname = ty.name();
                            // SQLite's stored type is dynamic — try
                            // integer / float / text in order.
                            if let Ok(i) = r.try_get::<i64, _>(i) {
                                return Json::from(i);
                            }
                            if let Ok(f) = r.try_get::<f64, _>(i) {
                                return Json::from(f);
                            }
                            if let Ok(s) = r.try_get::<String, _>(i) {
                                return Json::from(s);
                            }
                            Json::String(format!("<unsupported:{tyname}>"))
                        }
                    })
                    .unwrap_or(Json::Null);
                obj.insert(col.name().to_string(), value);
            }
            json_rows.push(Json::Object(obj));
        }

        let dataset_json = serde_json::json!({
            "schema": { "columns": columns },
            "rows": json_rows,
            "coverage": {
                "raw":      { "samples_expected": 1, "samples_present": 1, "confidence": 1.0 },
                "effective":{ "confidence": 1.0, "penalty_chain": [] },
                "quality_flags": [],
            },
            "tz": TimeZoneId::utc().as_str(),
            "window": serde_json::Value::Null,
            "rule_id": rule_id.to_string(),
        });

        let mut out = SlotMap::new();
        out.insert(DATASET_SLOT.to_owned(), SlotValue::Json(dataset_json));
        // For pipelines that want a verdict instead (rule.sql can
        // also assert), we surface a Healthy verdict carrying the
        // row count as evidence. The caller picks the slot it
        // consumes.
        let rule_id_clone = rule_id.clone();
        let summary = format!("rule.sql: {n} row(s)", n = rows.len());
        let v = Verdict::new(rule_id_clone, Utc::now(), Severity::Healthy, summary)
            .with_tags(Tags::empty());
        out.insert(
            VERDICT_SLOT.to_owned(),
            SlotValue::Json(serde_json::to_value(&v).expect("Verdict serialises")),
        );
        Ok(out)
    }
}

fn emit_error(rule_id: RuleId, kind: RuleErrorKind, summary: impl Into<String>) -> SlotMap {
    let mut cov = Coverage::full_point();
    cov.quality_flags.push(rule_error_flag(kind));
    let v = Verdict::error(rule_id, Utc::now(), summary)
        .with_coverage(cov)
        .with_tags(Tags::empty());
    let mut out = SlotMap::new();
    out.insert(
        VERDICT_SLOT.to_owned(),
        SlotValue::Json(serde_json::to_value(&v).expect("Verdict serialises")),
    );
    out
}
