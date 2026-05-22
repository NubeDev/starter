//! `starter.flow.rule.derive` — the marker node kind for inline,
//! anonymous, one-shot derivations (Insights SCOPE R-ins-3).
//!
//! Registered derivation rules use `rule.rust` / `rule.rhai` / `rule.sql`
//! directly; this node kind exists for authoring ergonomics — a
//! pipeline that wants a "data-transform step labelled clearly"
//! without minting a `RuleId` reaches for `rule.derive`. The body
//! is a Rhai script run under the locked sandbox (R-ins-4); the
//! script receives the upstream `dataset` as a parameter and must
//! return a Rhai array of rows.
//!
//! Input slots:
//! - `script` ([`SlotValue::String`], required) — Rhai source.
//! - `dataset` ([`SlotValue::Json`], required) — upstream
//!   dataset JSON `{ "rows": [...], "tz": "...", "coverage": ... }`.
//! - `confidence_penalty` ([`SlotValue::Json`], optional) — f32 in
//!   `[0, 1]`; applied via [`crate::penalty::apply_derivation_penalty`].
//! - `rule_id` ([`SlotValue::String`], optional) — explicit id;
//!   defaults to `anon.<blake3-prefix>` per D4.
//!
//! Output slot:
//! - `dataset` ([`SlotValue::Json`]) — derived dataset JSON.

use std::sync::Arc;

use async_trait::async_trait;
use rhai::{Dynamic, Scope};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    Coverage, Dataset, DatasetSchema, RuleSchema, TimeZoneId, VecDatasetRows,
};

use crate::nodes::rule_rhai::anon_rule_id;
use crate::penalty::apply_derivation_penalty;
use crate::rhai_sandbox::make_engine;

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.rule.derive";

/// Required input slot: Rhai source.
pub const SCRIPT_SLOT: &str = "script";

/// Required input slot: upstream dataset JSON.
pub const DATASET_SLOT: &str = "dataset";

/// Optional input slot: per-derivation confidence penalty.
pub const PENALTY_SLOT: &str = "confidence_penalty";

/// Optional input slot: explicit rule id (`ns.name@major`).
pub const RULE_ID_SLOT: &str = "rule_id";

/// Output slot: serialised derived dataset.
pub const OUT_DATASET_SLOT: &str = "dataset";

/// Body for `starter.flow.rule.derive`.
pub struct RuleDeriveNode {
    kind: KindId,
}

impl RuleDeriveNode {
    /// Construct a derive node body.
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is valid"),
        }
    }
}

impl Default for RuleDeriveNode {
    fn default() -> Self {
        Self::new()
    }
}

fn json_to_dynamic(v: &serde_json::Value) -> Dynamic {
    match v {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let v: rhai::Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(v)
        }
        serde_json::Value::Object(obj) => {
            let mut m = rhai::Map::new();
            for (k, val) in obj {
                m.insert(k.as_str().into(), json_to_dynamic(val));
            }
            Dynamic::from(m)
        }
    }
}

fn dynamic_to_json(v: Dynamic) -> serde_json::Value {
    if let Some(b) = v.clone().try_cast::<bool>() {
        return serde_json::Value::Bool(b);
    }
    if let Some(i) = v.clone().try_cast::<i64>() {
        return serde_json::Value::from(i);
    }
    if let Some(f) = v.clone().try_cast::<f64>() {
        return serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Some(s) = v.clone().try_cast::<rhai::ImmutableString>() {
        return serde_json::Value::String(s.to_string());
    }
    if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
        return serde_json::Value::Array(arr.into_iter().map(dynamic_to_json).collect());
    }
    if let Some(map) = v.clone().try_cast::<rhai::Map>() {
        let mut obj = serde_json::Map::new();
        for (k, val) in map {
            obj.insert(k.to_string(), dynamic_to_json(val));
        }
        return serde_json::Value::Object(obj);
    }
    serde_json::Value::Null
}

#[async_trait]
impl NodeBehavior for RuleDeriveNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let script = match input.remove(SCRIPT_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Err(NodeError::InvalidInput(
                    "rule.derive: `script` required".into(),
                ))
            }
        };
        let dataset = match input.remove(DATASET_SLOT) {
            Some(SlotValue::Json(j)) => j,
            _ => {
                return Err(NodeError::InvalidInput(
                    "rule.derive: `dataset` required".into(),
                ))
            }
        };
        let penalty = match input.remove(PENALTY_SLOT) {
            Some(SlotValue::Json(serde_json::Value::Number(n))) => n.as_f64().map(|f| f as f32),
            _ => None,
        };
        let rule_id = match input.remove(RULE_ID_SLOT) {
            Some(SlotValue::String(s)) => {
                crate::nodes::rule_rust::parse_rule_id(&s).unwrap_or_else(|| anon_rule_id(&script))
            }
            _ => anon_rule_id(&script),
        };

        let engine = make_engine(None);
        let mut scope = Scope::new();
        scope.push("dataset", json_to_dynamic(&dataset));
        let out_dyn = engine
            .eval_with_scope::<Dynamic>(&mut scope, &script)
            .map_err(|e| NodeError::Backend(format!("rule.derive: {e}")))?;

        // Expect script to return an array of rows (objects).
        let rows = match dynamic_to_json(out_dyn) {
            serde_json::Value::Array(a) => a,
            other => {
                return Err(NodeError::Backend(format!(
                    "rule.derive: script must return an array; got {other:?}"
                )))
            }
        };

        // Reconstruct dataset shell from upstream and apply penalty.
        let tz = dataset
            .get("tz")
            .and_then(|v| v.as_str())
            .map(|s| TimeZoneId::new(s.to_owned()))
            .unwrap_or_else(TimeZoneId::utc);
        let coverage: Coverage = dataset
            .get("coverage")
            .and_then(|c| serde_json::from_value::<Coverage>(c.clone()).ok())
            .unwrap_or_else(Coverage::full_point);
        let columns: Vec<String> = rows
            .first()
            .and_then(|r| r.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        let mut ds = Dataset::from_parts(
            DatasetSchema::new(columns),
            Arc::new(VecDatasetRows::new(rows)),
            coverage,
            tz,
            None,
        );

        if let Some(p) = penalty {
            let schema = RuleSchema::derivation(rule_id.clone()).with_confidence_penalty(p);
            ds = apply_derivation_penalty(ds, &schema);
        }

        let mut out = SlotMap::new();
        out.insert(
            OUT_DATASET_SLOT.to_owned(),
            SlotValue::Json(serde_json::json!({
                "rule_id": format!("{rule_id}"),
                "tz": ds.tz.as_str(),
                "coverage": ds.coverage,
                "rows": ds.rows.snapshot(),
                "schema": { "columns": ds.schema.columns },
            })),
        );
        Ok(out)
    }
}
