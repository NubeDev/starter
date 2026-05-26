//! Phase 3 smoke — reproduces the HVAC row AND the bills-
//! reconciliation row of the SCOPE use-case sanity check
//! (lines 1178–1179) end-to-end, against the new Phase 3 pieces:
//!
//! - `rule.derive` (inline Rhai derivation under the locked sandbox)
//! - `align` (multi-source frame, D8 NodeId audit identity)
//! - `StreamingDatasetRows` (D1 streaming impl)
//! - `DerivationCache` (tier-3 materialisation, sqlite-gated)
//! - `rule.ai-check` (AI as in-line judge — R-ins-10)
//! - `rule.ai-debug` (AI explains the failing rule — R-ins-10)
//! - auto-tagging (`starter.ai-check`, `starter.ai-debug`,
//!   `starter.ai-model:*`)
//! - model-family pinning + per-Verdict exact-model evidence
//! - onboarding-backfill cache-warming contract
//! - HVAC pack: pmv-comfort + setpoint-drift + short-cycle
//!
//! The two pipelines share a shape — trigger → align → derive →
//! assertions → verdict.join → ai-check → branch on severity →
//! {gate → notify} or {ai-debug → notify(ops)}. The smoke exercises
//! the node bodies in sequence; the engine's branch/gate/retry are
//! flow-engine concerns (R-ins-9), tested in starter-flow.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;
use starter_spi::insights::{Severity, Verdict};

use starter_insights::ai::ModelFamily;
use starter_insights::nodes::align::{
    AlignNode, FRAME_SECS_SLOT, FRAME_SLOT, GAP_POLICY_SLOT, SOURCES_SLOT, TZ_SLOT,
};
use starter_insights::nodes::rule_ai_check::{
    AiJudge, AiJudgePrompt, AiJudgement, RuleAiCheckNode, RULE_ID_SLOT as AI_CHECK_RULE_ID_SLOT,
    UPSTREAM_SLOT,
};
use starter_insights::nodes::rule_ai_debug::{
    AiDebugPrompt, AiDebugResponse, AiDebugger, RuleAiDebugNode, DIAGNOSIS_SLOT, ERROR_VERDICT_SLOT,
};
use starter_insights::nodes::rule_derive::{
    RuleDeriveNode, DATASET_SLOT, OUT_DATASET_SLOT, SCRIPT_SLOT as DERIVE_SCRIPT,
};
use starter_insights::nodes::rule_rust::{RuleRustNode, PARAMS_SLOT, RULE_ID_SLOT};
use starter_insights::nodes::verdict_join::{JoinMode, VerdictJoinNode};
use starter_insights::nodes::VERDICT_SLOT;
use starter_insights::RuleRegistry;

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

fn ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
    NodeCtx::new(
        RunId::new(),
        node,
        cancel,
        SkillSelection::NONE,
        &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
    )
}

fn decode(out: &SlotMap, slot: &str) -> serde_json::Value {
    match out.get(slot) {
        Some(SlotValue::Json(j)) => j.clone(),
        other => panic!("expected JSON slot `{slot}`, got {other:?}"),
    }
}

fn decode_verdict(out: &SlotMap) -> Verdict {
    serde_json::from_value(decode(out, VERDICT_SLOT)).expect("verdict decodes")
}

// ------------------------------------------------------------------
// Deterministic fake AiJudge / AiDebugger (no network, no provider
// SDK; satisfies the R-ins-5 CI dep-tree gate by construction).
// ------------------------------------------------------------------

struct StubJudge {
    family: ModelFamily,
    exact_model: String,
    verdict_severity: Severity,
    summary: String,
}

#[async_trait]
impl AiJudge for StubJudge {
    fn family(&self) -> &ModelFamily {
        &self.family
    }
    async fn judge(&self, _prompt: AiJudgePrompt) -> Result<AiJudgement, String> {
        Ok(AiJudgement {
            severity: self.verdict_severity,
            summary: self.summary.clone(),
            exact_model: self.exact_model.clone(),
        })
    }
}

struct StubDebugger {
    family: ModelFamily,
    exact_model: String,
}

#[async_trait]
impl AiDebugger for StubDebugger {
    fn family(&self) -> &ModelFamily {
        &self.family
    }
    async fn diagnose(&self, prompt: AiDebugPrompt) -> Result<AiDebugResponse, String> {
        Ok(AiDebugResponse {
            summary: format!(
                "stub diagnosis for {}: {}",
                prompt.error_verdict.rule_id, prompt.error_verdict.summary
            ),
            likely_cause: "input_missing".into(),
            suggested_fix: "supply the missing param; re-run".into(),
            confidence: 0.7,
            exact_model: self.exact_model.clone(),
        })
    }
}

// ------------------------------------------------------------------
// HVAC row — comfort vs cost.
// ------------------------------------------------------------------

#[tokio::test]
async fn hvac_row_pmv_setpoint_short_cycle_with_ai_judge_and_debug() {
    let mut rules = RuleRegistry::new();
    for r in starter_ext_insights_hvac::rules() {
        rules.register(r).expect("hvac rules register");
    }
    let rules = Arc::new(rules);

    let rule_rust = RuleRustNode::new(Arc::clone(&rules));
    let id = NodeId::new("hvac.smoke").unwrap();
    let cancel = NoCancel;

    // pmv 0.9 → Warn (outside default band).
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("hvac.pmv.comfort@1".into()),
    );
    input.insert(
        PARAMS_SLOT.into(),
        SlotValue::Json(serde_json::json!({"pmv": 0.9})),
    );
    let pmv = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(pmv.severity, Severity::Warn);
    assert!(pmv.tags.get("domain").is_some());

    // setpoint drift 23.0 vs 21.0, tolerance 1.0 → Warn.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("hvac.setpoint.drift@1".into()),
    );
    input.insert(
        PARAMS_SLOT.into(),
        SlotValue::Json(serde_json::json!({
            "measured_c": 23.0, "setpoint_c": 21.0, "tolerance_c": 1.0,
        })),
    );
    let drift = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(drift.severity, Severity::Warn);

    // short-cycling 12 > 6 default → Critical.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("hvac.short-cycle@1".into()),
    );
    input.insert(
        PARAMS_SLOT.into(),
        SlotValue::Json(serde_json::json!({"cycles": 12})),
    );
    let cyc = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(cyc.severity, Severity::Critical);

    // verdict.join (weighted) — joined severity is at least Warn.
    let join = VerdictJoinNode::new(
        starter_spi::insights::RuleId::new("acme.hvac", "pipeline", 1),
        JoinMode::Weighted(vec![
            ("pmv".into(), 1.0),
            ("drift".into(), 1.0),
            ("cyc".into(), 2.0),
        ]),
    );
    let mut join_in = SlotMap::new();
    join_in.insert(
        "pmv".into(),
        SlotValue::Json(serde_json::to_value(&pmv).unwrap()),
    );
    join_in.insert(
        "drift".into(),
        SlotValue::Json(serde_json::to_value(&drift).unwrap()),
    );
    join_in.insert(
        "cyc".into(),
        SlotValue::Json(serde_json::to_value(&cyc).unwrap()),
    );
    let joined = decode_verdict(&join.invoke(ctx(&id, &cancel), join_in).await.unwrap());
    assert!(
        joined.severity.rank() >= Severity::Warn.rank(),
        "joined severity should reflect the worst input"
    );

    // rule.ai-check — AI judge weighs the joined verdict and lands
    // on Critical (stubbed). Auto-tags + exact-model evidence.
    let judge = StubJudge {
        family: ModelFamily::new("anthropic", "claude-opus-4"),
        exact_model: "claude-opus-4-20250514".into(),
        verdict_severity: Severity::Critical,
        summary: "real comfort + cycle incident".into(),
    };
    let check = RuleAiCheckNode::new(Arc::new(judge));
    let mut ai_in = SlotMap::new();
    ai_in.insert(
        AI_CHECK_RULE_ID_SLOT.into(),
        SlotValue::String("acme.hvac.judge@1".into()),
    );
    ai_in.insert(
        UPSTREAM_SLOT.into(),
        SlotValue::Json(serde_json::json!([joined])),
    );
    let ai_verdict = decode_verdict(&check.invoke(ctx(&id, &cancel), ai_in).await.unwrap());
    assert_eq!(ai_verdict.severity, Severity::Critical);
    assert!(
        ai_verdict.tags.get("starter.ai-check").is_some(),
        "auto-tag starter.ai-check present"
    );
    assert!(
        ai_verdict
            .tags
            .0
            .keys()
            .any(|k| k.starts_with("starter.ai-model")),
        "auto-tag starter.ai-model:* present"
    );
    assert!(
        ai_verdict
            .evidence
            .iter()
            .any(|e| e.value["kind"] == "ai-check.model"
                && e.value["exact_model"] == "claude-opus-4-20250514"),
        "per-Verdict exact-model evidence present"
    );

    // rule.ai-debug — Error branch. Construct an Error verdict and
    // route through ai-debug; assert the diagnosis is well-formed.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("hvac.setpoint.drift@1".into()),
    );
    // Missing measured_c forces a Severity::Error.
    input.insert(PARAMS_SLOT.into(), SlotValue::Json(serde_json::json!({})));
    let err_v = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(err_v.severity, Severity::Error);

    let dbg = RuleAiDebugNode::new(Arc::new(StubDebugger {
        family: ModelFamily::new("anthropic", "claude-opus-4"),
        exact_model: "claude-opus-4-20250514".into(),
    }));
    let mut dbg_in = SlotMap::new();
    dbg_in.insert(
        ERROR_VERDICT_SLOT.into(),
        SlotValue::Json(serde_json::to_value(&err_v).unwrap()),
    );
    let dbg_out = dbg.invoke(ctx(&id, &cancel), dbg_in).await.unwrap();
    let diag = decode(&dbg_out, DIAGNOSIS_SLOT);
    assert_eq!(diag["likely_cause"], "input_missing");
    assert_eq!(diag["exact_model"], "claude-opus-4-20250514");
}

// ------------------------------------------------------------------
// Bills-reconciliation row — the ugly one. Align multi-source +
// inline Rhai derive + custom Rhai rule + AI judge + AI debugger on
// the error edge.
// ------------------------------------------------------------------

#[tokio::test]
async fn bills_reconciliation_row_end_to_end() {
    let id = NodeId::new("bills.smoke").unwrap();
    let cancel = NoCancel;

    // (1) align — combine meter + weather + tariff + occupancy. The
    //     align node carries the D8 NodeId audit identity in its
    //     output frame.
    let align = AlignNode::new();
    let mut align_in = SlotMap::new();
    align_in.insert(
        SOURCES_SLOT.into(),
        SlotValue::Json(serde_json::json!({
            "meter": [
                {"ts": "2024-06-01T00:00:00Z", "value": 1.1},
                {"ts": "2024-06-01T00:15:00Z", "value": 1.2},
                {"ts": "2024-06-01T00:30:00Z", "value": 1.3},
                {"ts": "2024-06-01T00:45:00Z", "value": 1.0},
            ],
            "weather": [
                {"ts": "2024-06-01T00:00:00Z", "value": 12.0},
                {"ts": "2024-06-01T00:15:00Z", "value": 12.5},
                {"ts": "2024-06-01T00:30:00Z", "value": 13.0},
                {"ts": "2024-06-01T00:45:00Z", "value": 13.5},
            ],
            "tariff":   [{"ts": "2024-06-01T00:00:00Z", "value": 0.18}],
            "occupancy":[{"ts": "2024-06-01T00:00:00Z", "value": 1}],
        })),
    );
    align_in.insert(FRAME_SECS_SLOT.into(), SlotValue::Int(15 * 60));
    align_in.insert(TZ_SLOT.into(), SlotValue::String("Europe/London".into()));
    align_in.insert(GAP_POLICY_SLOT.into(), SlotValue::String("mark_gap".into()));
    let frame_out = align.invoke(ctx(&id, &cancel), align_in).await.unwrap();
    let frame = decode(&frame_out, FRAME_SLOT);
    assert_eq!(frame["node_id"], "starter.align.tumble@1");
    assert_eq!(frame["tz"], "Europe/London");
    assert!(frame["sources"]["meter"]["rows"].is_array());

    // (2) rule.derive — inline Rhai over the meter rows; despike via
    //     simple clamp.  Confidence_penalty=0.95 propagates.
    let derive = RuleDeriveNode::new();
    let meter_ds = serde_json::json!({
        "tz": "Europe/London",
        "rows": frame["sources"]["meter"]["rows"],
        "coverage": null,
    });
    let mut derive_in = SlotMap::new();
    derive_in.insert(
        DERIVE_SCRIPT.into(),
        SlotValue::String(
            r#"
                let out = [];
                for row in dataset.rows {
                    let v = row.value;
                    if v > 1.25 { v = 1.25 }
                    out.push(#{ ts: row.ts, value: v });
                }
                out
            "#
            .into(),
        ),
    );
    derive_in.insert(DATASET_SLOT.into(), SlotValue::Json(meter_ds));
    derive_in.insert(
        "confidence_penalty".into(),
        SlotValue::Json(serde_json::json!(0.95)),
    );
    let derived_out = derive.invoke(ctx(&id, &cancel), derive_in).await.unwrap();
    let derived = decode(&derived_out, OUT_DATASET_SLOT);
    let rows = derived["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r["value"].as_f64().unwrap() <= 1.25));
    let penalty_chain = derived["coverage"]["effective"]["penalty_chain"]
        .as_array()
        .unwrap();
    assert_eq!(penalty_chain.len(), 1, "rule.derive applied the penalty");

    // (3) custom Rhai rule — tariff window overrun (the operator's
    //     single custom rule). Runs under the locked R-ins-4 sandbox
    //     via rule.rhai inline (covered in energy_smoke); here we
    //     stand in for the verdict it would emit.
    let custom = Verdict::warn(
        starter_spi::insights::RuleId::new("acme", "tariff-window-overrun", 1),
        chrono::Utc::now(),
        "peak band overrun: 5 h > 3 h",
    );

    // Reusable assertion rule — baseline-deviation from energy pack
    // (proves Phase 2 packs compose alongside Phase 3 nodes).
    let mut rules = RuleRegistry::new();
    for r in starter_ext_insights_energy::rules() {
        rules.register(r).expect("energy rules register");
    }
    let rules = Arc::new(rules);
    let rule_rust = RuleRustNode::new(Arc::clone(&rules));
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
    let baseline = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(baseline.severity, Severity::Warn);

    // (4) verdict.join (weighted) → AI judge → final verdict tagged
    //     starter.ai-check / starter.ai-model:*.
    let join = VerdictJoinNode::new(
        starter_spi::insights::RuleId::new("acme.energy", "bills-pipeline", 1),
        JoinMode::Weighted(vec![("custom".into(), 1.0), ("baseline".into(), 2.0)]),
    );
    let mut join_in = SlotMap::new();
    join_in.insert(
        "custom".into(),
        SlotValue::Json(serde_json::to_value(&custom).unwrap()),
    );
    join_in.insert(
        "baseline".into(),
        SlotValue::Json(serde_json::to_value(&baseline).unwrap()),
    );
    let joined = decode_verdict(&join.invoke(ctx(&id, &cancel), join_in).await.unwrap());
    assert!(joined.severity.rank() >= Severity::Warn.rank());

    let judge = StubJudge {
        family: ModelFamily::new("anthropic", "claude-opus-4"),
        exact_model: "claude-opus-4-20250514".into(),
        verdict_severity: Severity::Critical,
        summary: "bill anomaly confirmed".into(),
    };
    let check = RuleAiCheckNode::new(Arc::new(judge));
    let mut ai_in = SlotMap::new();
    ai_in.insert(
        AI_CHECK_RULE_ID_SLOT.into(),
        SlotValue::String("acme.bill-judge@1".into()),
    );
    ai_in.insert(
        UPSTREAM_SLOT.into(),
        SlotValue::Json(serde_json::json!([joined])),
    );
    let ai_verdict = decode_verdict(&check.invoke(ctx(&id, &cancel), ai_in).await.unwrap());
    assert_eq!(ai_verdict.severity, Severity::Critical);
    assert!(ai_verdict.tags.get("starter.ai-check").is_some());

    // (5) Force a Severity::Error on a sibling assertion → ai-debug
    //     emits a diagnosis.  This is the canonical error-edge
    //     pattern from R-ins-9.
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.into(),
        SlotValue::String("energy.peak.detect@1".into()),
    );
    input.insert(PARAMS_SLOT.into(), SlotValue::Json(serde_json::json!({})));
    let err_v = decode_verdict(&rule_rust.invoke(ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(err_v.severity, Severity::Error);

    let dbg = RuleAiDebugNode::new(Arc::new(StubDebugger {
        family: ModelFamily::new("anthropic", "claude-opus-4"),
        exact_model: "claude-opus-4-20250514".into(),
    }));
    let mut dbg_in = SlotMap::new();
    dbg_in.insert(
        ERROR_VERDICT_SLOT.into(),
        SlotValue::Json(serde_json::to_value(&err_v).unwrap()),
    );
    let dbg_out = dbg.invoke(ctx(&id, &cancel), dbg_in).await.unwrap();
    let diag = decode(&dbg_out, DIAGNOSIS_SLOT);
    assert!(
        diag["summary"]
            .as_str()
            .unwrap()
            .contains("energy.peak.detect@1"),
        "diagnosis names the failing rule"
    );
}

// ------------------------------------------------------------------
// Onboarding-backfill cache-warming contract.
// ------------------------------------------------------------------

#[tokio::test]
async fn onboarding_backfill_caps_and_flags_partial() {
    use chrono::Utc;
    use starter_insights::onboarding::{run_onboarding_backfill, OnboardingPlan};
    use starter_spi::insights::{RuleId, Verdict};

    let plan = OnboardingPlan::default_for(RuleId::new("acme", "warm", 1), Utc::now());
    let start = plan.window_start;
    // Drive past the D3 cap to prove truncation works through the
    // onboarding wrapper.
    let stream = (0..150_000_u32).map(|i| {
        Verdict::new(
            RuleId::new("acme", "warm", 1),
            start,
            Severity::Healthy,
            format!("v{i}"),
        )
    });
    let out = run_onboarding_backfill(plan, stream);
    assert!(
        matches!(
            out.event,
            starter_insights::backfill::BackfillEvent::Truncated { .. }
        ),
        "onboarding backfill truncated at the cap"
    );
    assert!(out.verdicts[0]
        .coverage
        .quality_flags
        .iter()
        .any(|f| f.id.name == "partial-onboarding"));
}

// ------------------------------------------------------------------
// StreamingDatasetRows — the heavy-end of the DatasetRows trait
// (D1).  Proves chunked iteration and snapshot consistency.
// ------------------------------------------------------------------

#[test]
fn streaming_dataset_rows_chunks_and_snapshots() {
    use starter_insights::streaming::StreamingDatasetRows;
    use starter_spi::insights::DatasetRows;
    let rows: Vec<serde_json::Value> = (0..23_000).map(|i| serde_json::json!({"i": i})).collect();
    let s = StreamingDatasetRows::from_rows(rows);
    assert_eq!(s.len(), 23_000);
    assert_eq!(s.chunk_count(), 3); // 10k + 10k + 3k
    let snap = DatasetRows::snapshot(&s);
    assert_eq!(snap.len(), 23_000);
    assert_eq!(snap[0]["i"], 0);
    assert_eq!(snap[22_999]["i"], 22_999);
}

// ------------------------------------------------------------------
// Skill bundles + AI auto-tagging are exposed.
// ------------------------------------------------------------------

#[test]
fn three_insights_skill_bundles_are_named() {
    use starter_insights::skills::{ALL, BUNDLE_EXPLAIN, BUNDLE_RULE_AUTHOR, BUNDLE_TUNER};
    assert_eq!(
        ALL,
        &[BUNDLE_RULE_AUTHOR, BUNDLE_EXPLAIN, BUNDLE_TUNER],
        "three skill bundles, canonical order"
    );
    // Confirm the static metadata directories exist at workspace root
    // — per agent R4 the loader expects `skills/<id>/SKILL.md`.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for bundle in ALL {
        let path = root.join("skills").join(bundle).join("SKILL.md");
        assert!(
            path.exists(),
            "skill bundle file missing: {}",
            path.display()
        );
    }
}

// ------------------------------------------------------------------
// Derivation cache (tier-3 materialisation) under the sqlite gate.
// ------------------------------------------------------------------

#[cfg(feature = "sqlite")]
mod cache_tests {
    use chrono::{TimeZone, Utc};
    use starter_insights::cache::DerivationCache;
    use starter_insights::sqlite::INSIGHTS_MIGRATION_SOURCE;
    use starter_spi::insights::{
        Coverage, Dataset, DatasetSchema, RuleId, TimeZoneId, VecDatasetRows, Window,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn cache_put_get_invalidate() {
        let pool = starter_store_sqlite::testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .unwrap();

        let cache = DerivationCache::new(pool.clone());
        let id = RuleId::new("energy", "meter.fill-gaps", 2);
        let start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 6, 1, 1, 0, 0).unwrap();
        let ds = Dataset::from_parts(
            DatasetSchema::new(["ts", "value"]),
            Arc::new(VecDatasetRows::new(vec![
                serde_json::json!({"ts": "2024-06-01T00:00:00Z", "value": 1.0}),
            ])),
            Coverage::full_point(),
            TimeZoneId::utc(),
            Some(Window::new(start, end)),
        );

        cache.put(&id, start, end, &ds).await.unwrap();
        assert_eq!(cache.count().await.unwrap(), 1);

        let hit = cache.get(&id, start).await.unwrap().expect("cache hit");
        assert_eq!(hit.rows.len(), 1);

        // Idempotent overwrite.
        cache.put(&id, start, end, &ds).await.unwrap();
        assert_eq!(cache.count().await.unwrap(), 1);

        let n = cache.invalidate(&id).await.unwrap();
        assert_eq!(n, 1);
        assert!(cache.get(&id, start).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn cache_invalidate_rule_version_wipes_all_majors() {
        let pool = starter_store_sqlite::testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .unwrap();
        let cache = DerivationCache::new(pool.clone());
        let v1 = RuleId::new("energy", "meter.fill-gaps", 1);
        let v2 = RuleId::new("energy", "meter.fill-gaps", 2);
        let start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let end = start + chrono::Duration::hours(1);
        let ds = Dataset::from_parts(
            DatasetSchema::new(Vec::<String>::new()),
            Arc::new(VecDatasetRows::empty()),
            Coverage::full_point(),
            TimeZoneId::utc(),
            Some(Window::new(start, end)),
        );
        cache.put(&v1, start, end, &ds).await.unwrap();
        cache.put(&v2, start, end, &ds).await.unwrap();
        assert_eq!(cache.count().await.unwrap(), 2);
        let n = cache
            .invalidate_rule_version("energy", "meter.fill-gaps")
            .await
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(cache.count().await.unwrap(), 0);
    }
}

// ------------------------------------------------------------------
// D5 — durable fix: rollup drain now RE-AGGREGATES invalidated
// windows.  Prior to stage 3 the drain only DELETED queued rows
// without re-folding the underlying verdicts, leaving pre-fixup
// totals in `verdict_rollup`. This test asserts the new behaviour:
//
// 1. seed two healthy verdicts → tick rollup → ungrouped count is
//    `(2, 0, 0, 0, 0)`.
// 2. append a critical verdict in the same window (representing a
//    "tariff fixup" mutation).
// 3. enqueue an invalidation for that window.
// 4. tick rollup → drain re-folds → ungrouped count is
//    `(2, 0, 0, 1, 0)`.
// 5. invalidation queue drained to empty.
// 6. D5 retroactive flag attached on the third verdict (engine seam
//    via `retroactive::attach_retroactive_flag`).
// ------------------------------------------------------------------

#[cfg(feature = "sqlite")]
mod d5_durable_fix {

    use chrono::{TimeZone, Utc};
    use starter_insights::retroactive::{attach_retroactive_flag, MutationWatermarks};
    use starter_insights::rollups::{RollupEngine, WindowClass};
    use starter_insights::sqlite::{VerdictStore, INSIGHTS_MIGRATION_SOURCE};
    use starter_spi::insights::{RuleId, RuleSchema, Severity, Verdict, Window};

    #[tokio::test]
    async fn rollup_drain_re_aggregates_after_input_mutation() {
        let pool = starter_store_sqlite::testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .unwrap();
        let store = VerdictStore::new(pool.clone());
        let engine = RollupEngine::new(pool.clone());
        let tz: chrono_tz::Tz = "Europe/London".parse().unwrap();
        let rule = RuleId::new("energy", "usage.baseline-deviation", 1);

        // Pick a wall-clock instant deep enough into the day that the
        // bucket boundary is unambiguous in both UTC and London tz.
        let at = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        store
            .append(&Verdict::new(rule.clone(), at, Severity::Healthy, "a"))
            .await
            .unwrap();
        store
            .append(&Verdict::new(rule.clone(), at, Severity::Healthy, "b"))
            .await
            .unwrap();
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
        let (h, _, _, c, _) = engine
            .read_ungrouped_count(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                start,
            )
            .await
            .unwrap();
        assert_eq!((h, c), (2, 0), "initial rollup counts");

        // Simulate the tariff-fixup: a third verdict (now Critical)
        // and an invalidation for the affected window.
        store
            .append(&Verdict::new(rule.clone(), at, Severity::Critical, "fixup"))
            .await
            .unwrap();
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
        assert_eq!(
            engine
                .pending_invalidations("energy", "usage.baseline-deviation", 1, WindowClass::Day)
                .await
                .unwrap(),
            1,
            "invalidation enqueued"
        );

        // Tick — the drain MUST re-fold the window and clear the
        // queue. This is the durable D5 fix.
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
        let (h, _, _, c, _) = engine
            .read_ungrouped_count(
                "energy",
                "usage.baseline-deviation",
                1,
                WindowClass::Day,
                start,
            )
            .await
            .unwrap();
        assert_eq!(
            (h, c),
            (2, 1),
            "drain re-aggregated the window: 2 healthy + 1 critical"
        );
        assert_eq!(
            engine
                .pending_invalidations("energy", "usage.baseline-deviation", 1, WindowClass::Day)
                .await
                .unwrap(),
            0,
            "invalidation queue drained"
        );
    }

    #[test]
    fn d5_retroactive_flag_attached_for_retroactive_rules() {
        let wm = MutationWatermarks::new();
        wm.record(
            "source.tariff",
            Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap(),
        );
        let schema = RuleSchema::derivation(RuleId::new("energy", "tariff.apply-retroactive", 1))
            .retroactive();
        let mut v = Verdict::new(
            RuleId::new("energy", "tariff.apply-retroactive", 1),
            Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            Severity::Warn,
            "bill anomaly",
        )
        .with_window(Window::new(
            Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 6, 2, 0, 0, 0).unwrap(),
        ));
        assert!(attach_retroactive_flag(&mut v, &schema, &wm));
        assert!(v
            .coverage
            .quality_flags
            .iter()
            .any(|f| f.id.name == "retroactive-correction"));
    }
}
