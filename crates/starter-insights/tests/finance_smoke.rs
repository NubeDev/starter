//! Phase 4 — finance pack smoke.
//!
//! Exercises the three rules from `starter-ext-insights-finance`
//! (`finance.tx.z-score@1`, `finance.tx.isolation-forest-light@1`,
//! `finance.tx.duplicate@1`) through the `RuleRustNode` so the
//! same registration path the IoT / Energy / HVAC packs use is
//! covered for the new pack — per R-ins-1 a rule is just a node and
//! finance is no exception.
//!
//! Also verifies the `finance.quality.*` flag descriptors are
//! contributed via the same extension surface every other pack
//! uses (R-ins-11).

use std::sync::Arc;

use serde_json::json;
use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;
use starter_spi::insights::{QualityFlagId, Severity, Verdict};

use starter_insights::nodes::rule_rust::{RuleRustNode, PARAMS_SLOT, RULE_ID_SLOT};
use starter_insights::nodes::VERDICT_SLOT;
use starter_insights::{QualityFlagRegistry, RuleRegistry};

struct NoCancel;
impl Cancel for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
    NodeCtx::new(RunId::new(), node, cancel, SkillSelection::NONE, &starter_flow_spi::state::NOOP_NODE_STATE_STORE)
}

fn decode_verdict(out: &SlotMap) -> Verdict {
    match out.get(VERDICT_SLOT) {
        Some(SlotValue::Json(j)) => serde_json::from_value(j.clone()).expect("verdict decodes"),
        other => panic!("expected JSON `{VERDICT_SLOT}`, got {other:?}"),
    }
}

fn registry() -> Arc<RuleRegistry> {
    let mut reg = RuleRegistry::new();
    for r in starter_ext_insights_finance::rules() {
        reg.register(r).expect("register finance rule");
    }
    Arc::new(reg)
}

async fn invoke(reg: Arc<RuleRegistry>, rule_id: &str, params: serde_json::Value) -> Verdict {
    let node = RuleRustNode::new(reg);
    let nid = NodeId::new("finance.test.rule").expect("valid node id");
    let cancel = NoCancel;
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.to_owned(),
        SlotValue::String(rule_id.to_owned()),
    );
    input.insert(PARAMS_SLOT.to_owned(), SlotValue::Json(params));
    let out = node
        .invoke(make_ctx(&nid, &cancel), input)
        .await
        .expect("rule.rust body returns Ok (failures are verdicts)");
    decode_verdict(&out)
}

#[tokio::test]
async fn z_score_critical_on_extreme_outlier() {
    // Population: 100 samples around 100±5; the test value 500 is
    // far above 5σ.
    let population: Vec<f64> = (0..100).map(|i| 100.0 + ((i % 10) as f64 - 5.0)).collect();
    let v = invoke(
        registry(),
        "finance.tx.z-score@1",
        json!({
            "amount": 500.0,
            "population": population,
            "threshold_sigma": 3.0,
            "critical_sigma": 5.0,
        }),
    )
    .await;
    assert_eq!(v.severity, Severity::Critical, "summary={}", v.summary);
    assert_eq!(
        v.tags.get("domain").map(|tv| format!("{tv:?}")),
        Some(format!(
            "{:?}",
            starter_spi::insights::TagValue::Value("finance".to_owned())
        )),
        "domain:finance tag carried"
    );
}

#[tokio::test]
async fn z_score_healthy_inside_threshold() {
    let population: Vec<f64> = (0..100).map(|i| 100.0 + ((i % 10) as f64 - 5.0)).collect();
    let v = invoke(
        registry(),
        "finance.tx.z-score@1",
        json!({"amount": 101.0, "population": population, "threshold_sigma": 3.0}),
    )
    .await;
    assert_eq!(v.severity, Severity::Healthy);
}

#[tokio::test]
async fn isolation_forest_light_flags_outlier() {
    // Tight cluster + one far-outlier.
    let population: Vec<f64> = (0..50).map(|i| 50.0 + (i % 5) as f64 * 0.1).collect();
    let v = invoke(
        registry(),
        "finance.tx.isolation-forest-light@1",
        json!({"value": 9999.0, "population": population, "threshold_depth": 4, "seed": 42}),
    )
    .await;
    assert_eq!(v.severity, Severity::Warn, "summary={}", v.summary);
}

#[tokio::test]
async fn isolation_forest_light_is_deterministic_on_seed() {
    let population: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64) * 0.5).collect();
    let v1 = invoke(
        registry(),
        "finance.tx.isolation-forest-light@1",
        json!({"value": 200.0, "population": population.clone(), "seed": 12345}),
    )
    .await;
    let v2 = invoke(
        registry(),
        "finance.tx.isolation-forest-light@1",
        json!({"value": 200.0, "population": population, "seed": 12345}),
    )
    .await;
    assert_eq!(v1.severity, v2.severity);
    assert_eq!(v1.summary, v2.summary);
}

#[tokio::test]
async fn duplicate_tx_critical_within_bucket() {
    let v = invoke(
        registry(),
        "finance.tx.duplicate@1",
        json!({
            "transactions": [
                {"account": "A", "amount": 12.50, "ts": 1_700_000_000_i64},
                {"account": "A", "amount": 12.50, "ts": 1_700_000_020_i64},
                {"account": "B", "amount": 1.00,  "ts": 1_700_000_000_i64},
            ],
            "bucket_secs": 60,
        }),
    )
    .await;
    assert_eq!(v.severity, Severity::Critical);
    assert!(
        v.coverage
            .quality_flags
            .iter()
            .any(|f| f.id.namespace == "finance.quality" && f.id.name == "duplicate-timestamp"),
        "duplicate-timestamp quality flag must attach: {v:?}"
    );
}

#[tokio::test]
async fn duplicate_tx_healthy_when_outside_bucket() {
    let v = invoke(
        registry(),
        "finance.tx.duplicate@1",
        json!({
            "transactions": [
                {"account": "A", "amount": 12.50, "ts": 1_700_000_000_i64},
                {"account": "A", "amount": 12.50, "ts": 1_700_000_120_i64},
            ],
            "bucket_secs": 60,
        }),
    )
    .await;
    assert_eq!(v.severity, Severity::Healthy);
}

#[tokio::test]
async fn quality_flags_register_via_pack_seam() {
    let mut qreg = QualityFlagRegistry::new();
    for (id, desc, rem) in starter_ext_insights_finance::quality_flags() {
        qreg.register(
            id,
            starter_insights::registry::QualityFlagInfo::new(desc, rem),
        )
        .expect("register flag");
    }
    let dup = QualityFlagId::new("finance.quality", "duplicate-timestamp", 1);
    let fx = QualityFlagId::new("finance.quality", "fx-rate-stale", 1);
    assert!(qreg.get(&dup).is_some(), "duplicate-timestamp registered");
    assert!(qreg.get(&fx).is_some(), "fx-rate-stale registered");
}

#[tokio::test]
async fn finance_verdicts_are_pii_tagged() {
    // Finance assertions land on PII-sensitive data — every emitted
    // verdict must carry the `pii` flag so storage / routing react.
    let v = invoke(
        registry(),
        "finance.tx.duplicate@1",
        json!({"transactions": [], "bucket_secs": 60}),
    )
    .await;
    assert!(
        v.tags.get("pii").is_some(),
        "finance verdicts must be pii-tagged: tags={:?}",
        v.tags
    );
}
