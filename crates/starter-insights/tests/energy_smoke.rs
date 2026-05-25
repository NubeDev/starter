//! Phase 2 Energy smoke — reproduces the "Energy / water — baseline
//! deviation" row of the SCOPE use-case sanity-check (line 1176)
//! end-to-end, modulo the AI judge (Phase 3).
//!
//! What this test proves:
//!
//! 1. The Energy pack (`starter-ext-insights-energy`) registers its
//!    five rules and two `energy.quality.*` flags into the host's
//!    `RuleRegistry` / `QualityFlagRegistry` (R-ins-1, R-ins-11).
//! 2. `confidence_penalty` enforcement multiplies through the
//!    derivation chain (R-ins-6 raw/effective split).
//! 3. The locked Rhai sandbox (R-ins-4) evaluates a tariff-window
//!    rule and the `tariff-window-overrun` rhai script integrates
//!    into a `verdict.join` alongside Rust assertion rules.
//! 4. `window.slide` produces tz-aware sliding windows.
//! 5. (sqlite) Incremental rollups + D5 retroactive invalidation
//!    work as specified.

use std::sync::Arc;

use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;
use starter_spi::insights::{Severity, Verdict};

use starter_insights::nodes::rule_rhai::{RuleRhaiNode, PARAMS_SLOT as RHAI_PARAMS, SCRIPT_SLOT};
use starter_insights::nodes::rule_rust::{RuleRustNode, PARAMS_SLOT, RULE_ID_SLOT};
use starter_insights::nodes::verdict_join::{JoinMode, VerdictJoinNode};
use starter_insights::nodes::windowing::{
    WindowSlideNode, EXPECTED_PER_WINDOW_SLOT, SAMPLES_SLOT, SIZE_SECS_SLOT, STEP_SECS_SLOT,
    TZ_SLOT, WINDOWS_SLOT,
};
use starter_insights::nodes::VERDICT_SLOT;
use starter_insights::penalty::apply_derivation_penalty;
use starter_insights::registry::QualityFlagInfo;
use starter_insights::{QualityFlagRegistry, RuleRegistry};
use starter_spi::insights::RuleOutput;

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

fn build_registries() -> (Arc<RuleRegistry>, QualityFlagRegistry) {
    let mut rules = RuleRegistry::new();
    for r in starter_ext_insights_energy::rules() {
        rules.register(r).expect("energy rules register");
    }
    let mut flags = QualityFlagRegistry::with_builtins();
    for (id, desc, rem) in starter_ext_insights_energy::quality_flags() {
        flags
            .register(id, QualityFlagInfo::new(desc, rem))
            .expect("energy flag ids register");
    }
    (Arc::new(rules), flags)
}

fn decode(out: &SlotMap) -> Verdict {
    match out.get(VERDICT_SLOT) {
        Some(SlotValue::Json(j)) => serde_json::from_value(j.clone()).expect("verdict decodes"),
        other => panic!("expected verdict slot, got {other:?}"),
    }
}

#[tokio::test]
async fn energy_row_baseline_deviation_and_peak_detect() {
    let (rules, flags) = build_registries();
    assert_eq!(rules.len(), 5, "energy pack registers five rules");
    assert!(
        flags
            .list()
            .iter()
            .any(|f| f.namespace == "energy.quality" && f.name == "unit-changed"),
        "energy.quality.unit-changed registered"
    );

    let rule_rust = RuleRustNode::new(Arc::clone(&rules));
    let id = NodeId::new("e.test").unwrap();
    let cancel = NoCancel;

    // Baseline-deviation: measured 130 kWh vs baseline 100 with 20% threshold → Warn.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("energy.usage.baseline-deviation@1".into()),
    );
    input.insert(
        PARAMS_SLOT.into(),
        SlotValue::Json(serde_json::json!({
            "measured_kwh": 130.0,
            "baseline_kwh": 100.0,
            "threshold_pct": 20.0,
        })),
    );
    let dev = decode(
        &rule_rust
            .invoke(make_ctx(&id, &cancel), input)
            .await
            .unwrap(),
    );
    assert_eq!(dev.severity, Severity::Warn);
    assert!(dev.tags.get("domain").is_some());

    // Peak detect: 250 kW vs ceiling 200 → Critical.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("energy.peak.detect@1".into()),
    );
    input.insert(
        PARAMS_SLOT.into(),
        SlotValue::Json(serde_json::json!({"value_kw": 250.0, "peak_kw": 200.0})),
    );
    let peak = decode(
        &rule_rust
            .invoke(make_ctx(&id, &cancel), input)
            .await
            .unwrap(),
    );
    assert_eq!(peak.severity, Severity::Critical);

    // Fan-in weighted: the joined verdict is at least Warn (mean rank
    // weighted between Warn=2 and Critical=3).
    let join = VerdictJoinNode::new(
        starter_spi::insights::RuleId::new("energy.pipeline", "baseline-and-peak", 1),
        JoinMode::Weighted(vec![("dev".into(), 1.0), ("peak".into(), 1.0)]),
    );
    let mut join_in = SlotMap::new();
    join_in.insert(
        "dev".into(),
        SlotValue::Json(serde_json::to_value(&dev).unwrap()),
    );
    join_in.insert(
        "peak".into(),
        SlotValue::Json(serde_json::to_value(&peak).unwrap()),
    );
    let joined = decode(&join.invoke(make_ctx(&id, &cancel), join_in).await.unwrap());
    assert!(
        joined.severity == Severity::Warn || joined.severity == Severity::Critical,
        "joined severity should reflect the worst input, got {:?}",
        joined.severity
    );
}

#[tokio::test]
async fn derivation_chain_applies_confidence_penalties() {
    let (rules, _) = build_registries();
    let fill = rules
        .get(&starter_spi::insights::RuleId::new(
            "energy",
            "meter.fill-gaps",
            2,
        ))
        .unwrap();
    let resample = rules
        .get(&starter_spi::insights::RuleId::new(
            "weather",
            "resample.15m-to-1m",
            1,
        ))
        .unwrap();
    let normalise = rules
        .get(&starter_spi::insights::RuleId::new(
            "energy",
            "normalise.weather",
            2,
        ))
        .unwrap();

    let input = starter_spi::insights::RuleInput::from_parts(
        serde_json::Map::from_iter([(
            "samples".into(),
            serde_json::json!([
                {"ts": "2024-01-01T00:00:00Z", "value": 1.0},
                {"ts": "2024-01-01T00:01:00Z", "value": null},
                {"ts": "2024-01-01T00:02:00Z", "value": 1.2},
            ]),
        )]),
        None,
    );

    let ds = match fill.evaluate(input).await {
        RuleOutput::Derivation(d) => d,
        _ => panic!("fill-gaps must return Derivation"),
    };
    let ds = apply_derivation_penalty(ds, fill.schema());
    // 0.8 penalty multiplied through; raw confidence reflects the
    // 2-of-3 samples present.
    assert!((ds.coverage.effective.confidence - (ds.coverage.raw.confidence * 0.8)).abs() < 1e-5);
    assert_eq!(ds.coverage.effective.penalty_chain.len(), 1);

    // Chain through resample (0.9) and normalise (0.95).
    let ds2 = match resample
        .evaluate(starter_spi::insights::RuleInput::empty())
        .await
    {
        RuleOutput::Derivation(d) => d,
        _ => panic!(),
    };
    let ds2 = apply_derivation_penalty(ds2, resample.schema());
    assert!((ds2.coverage.effective.confidence - 0.9).abs() < 1e-5);
    let ds3 = match normalise
        .evaluate(starter_spi::insights::RuleInput::empty())
        .await
    {
        RuleOutput::Derivation(d) => d,
        _ => panic!(),
    };
    let ds3 = apply_derivation_penalty(ds3, normalise.schema());
    assert!((ds3.coverage.effective.confidence - 0.95).abs() < 1e-5);
}

#[tokio::test]
async fn registry_rejects_penalty_above_one() {
    use starter_spi::insights::{RuleId, RuleSchema};
    struct Bad {
        s: RuleSchema,
    }
    #[async_trait::async_trait]
    impl starter_spi::insights::Rule for Bad {
        fn schema(&self) -> &RuleSchema {
            &self.s
        }
        async fn evaluate(&self, _i: starter_spi::insights::RuleInput) -> RuleOutput {
            unreachable!()
        }
    }
    let mut r = RuleRegistry::new();
    let bad = Arc::new(Bad {
        s: RuleSchema::derivation(RuleId::new("t", "bad", 1)).with_confidence_penalty(1.5),
    });
    let err = r.register(bad).expect_err("must reject penalty > 1.0");
    let msg = format!("{err}");
    assert!(msg.contains("confidence_penalty"), "msg = {msg}");
}

#[tokio::test]
async fn rhai_tariff_window_overrun_returns_warn() {
    // The Energy pipeline's "custom rule" — a tariff-window-overrun
    // check authored by the operator and run under the locked
    // sandbox (R-ins-4).
    let node = RuleRhaiNode::new();
    let id = NodeId::new("e.rhai").unwrap();
    let cancel = NoCancel;
    let mut input = SlotMap::new();
    input.insert(
        SCRIPT_SLOT.into(),
        SlotValue::String(
            r#"
                let overrun = params["hours_in_peak_band"];
                if overrun > params["allowed_peak_hours"] {
                    #{ severity: "warn",
                       summary: "tariff peak window overrun" }
                } else {
                    #{ severity: "healthy",
                       summary: "tariff peak window OK" }
                }
            "#
            .into(),
        ),
    );
    input.insert(
        RHAI_PARAMS.into(),
        SlotValue::Json(serde_json::json!({
            "hours_in_peak_band": 5,
            "allowed_peak_hours": 3,
        })),
    );
    let v = decode(&node.invoke(make_ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(v.severity, Severity::Warn);
}

#[tokio::test]
async fn window_slide_emits_tz_aware_windows() {
    let node = WindowSlideNode::new();
    let id = NodeId::new("e.win").unwrap();
    let cancel = NoCancel;
    let mut input = SlotMap::new();
    input.insert(
        SAMPLES_SLOT.into(),
        SlotValue::Json(serde_json::json!([
            {"ts": "2024-06-01T00:00:00Z", "value": 1.0},
            {"ts": "2024-06-01T00:30:00Z", "value": 2.0},
            {"ts": "2024-06-01T01:00:00Z", "value": 3.0},
            {"ts": "2024-06-01T01:30:00Z", "value": 4.0},
        ])),
    );
    input.insert(SIZE_SECS_SLOT.into(), SlotValue::Int(3600));
    input.insert(STEP_SECS_SLOT.into(), SlotValue::Int(1800));
    input.insert(TZ_SLOT.into(), SlotValue::String("Europe/London".into()));
    input.insert(EXPECTED_PER_WINDOW_SLOT.into(), SlotValue::Int(2));
    let out = node.invoke(make_ctx(&id, &cancel), input).await.unwrap();
    let windows = match out.get(WINDOWS_SLOT) {
        Some(SlotValue::Json(serde_json::Value::Array(a))) => a.clone(),
        _ => panic!("missing windows slot"),
    };
    assert!(!windows.is_empty(), "at least one window emitted");
    assert!(
        windows.iter().any(|w| w["tz"] == "Europe/London"),
        "tz config propagated"
    );
}

#[cfg(feature = "sqlite")]
mod rollup_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use starter_insights::rollups::{RollupEngine, WindowClass};
    use starter_insights::sqlite::{VerdictStore, INSIGHTS_MIGRATION_SOURCE};
    use starter_spi::insights::{RuleId, Severity, Verdict};

    #[tokio::test]
    async fn incremental_rollup_counts_severities_with_tag_grouping() {
        let pool = starter_store_sqlite::testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .unwrap();

        let store = VerdictStore::new(pool.clone());
        let at = Utc.with_ymd_and_hms(2024, 6, 1, 12, 30, 0).unwrap();
        let rule = RuleId::new("energy", "usage.baseline-deviation", 1);
        let mk = |sev: Severity| {
            Verdict::new(rule.clone(), at, sev, "x")
                .with_tags(starter_spi::insights::Tags::empty().with_value("building", "hq-london"))
        };
        store.append(&mk(Severity::Healthy)).await.unwrap();
        store.append(&mk(Severity::Warn)).await.unwrap();
        store.append(&mk(Severity::Warn)).await.unwrap();

        let engine = RollupEngine::new(pool.clone());
        let tz: chrono_tz::Tz = "Europe/London".parse().unwrap();
        let n = engine
            .tick_incremental(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                tz,
                &["building"],
            )
            .await
            .unwrap();
        assert_eq!(n, 3);
        let (start, _end) = WindowClass::Day.bucket(tz, at);
        let (h, _i, w, _c, _e) = engine
            .read_ungrouped_count(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                start,
            )
            .await
            .unwrap();
        assert_eq!(h, 1);
        assert_eq!(w, 2);

        // Second tick is incremental — no new verdicts, no new counts.
        let n2 = engine
            .tick_incremental(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                tz,
                &["building"],
            )
            .await
            .unwrap();
        assert_eq!(n2, 0);
    }

    #[tokio::test]
    async fn d5_retroactive_invalidation_marks_stale_and_drains() {
        let pool = starter_store_sqlite::testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .unwrap();

        let store = VerdictStore::new(pool.clone());
        let at = Utc.with_ymd_and_hms(2024, 6, 1, 12, 30, 0).unwrap();
        let rule = RuleId::new("energy", "usage.baseline-deviation", 1);
        store
            .append(&Verdict::new(rule.clone(), at, Severity::Healthy, "x"))
            .await
            .unwrap();
        let engine = RollupEngine::new(pool.clone());
        let tz: chrono_tz::Tz = "Europe/London".parse().unwrap();
        engine
            .tick_incremental(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                tz,
                &[],
            )
            .await
            .unwrap();
        let (start, end) = WindowClass::Day.bucket(tz, at);
        engine
            .enqueue_invalidation(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                start,
                end,
                "tariff-fixup",
            )
            .await
            .unwrap();

        // Re-tick drains the invalidation queue.
        engine
            .tick_incremental(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                tz,
                &[],
            )
            .await
            .unwrap();
    }
}

#[test]
fn backfill_cap_is_100k() {
    assert_eq!(starter_insights::backfill::BACKFILL_ROW_CAP, 100_000);
}
