//! Phase B gate test — end-to-end proof that an extension-authored
//! anomaly rule fires against the cleaner's pure window walker.
//!
//! See [`rubix/docs/scope/extensions-north-star/PROGRESS.md`] Phase B
//! gate. The pipeline under test:
//!
//! 1. A bundle on disk declares
//!    `contributes.anomaly_rules: [{ id, tool_id }]`.
//! 2. [`starter_ext_host::Loader`] scans + validates + commits the
//!    bundle into an [`ExtensionRegistry`].
//! 3. [`rubix_agent::registry::collect_anomaly_rule_contributions`]
//!    walks the registry and projects every entry into a
//!    [`rubix_tools::cleaner::adapter::ContributedRule`].
//! 4. [`rubix_tools::cleaner::adapter::build_registry_with_contributions`]
//!    seeds the cleaner [`RuleRegistry`] with the three builtins and
//!    appends one `ToolAnomalyRule` per contribution, resolving the
//!    `tool_id` against the supplied `&[Arc<dyn Tool>]`.
//! 5. [`rubix_tools::cleaner::process_entity_window`] walks a
//!    synthetic L1 window and the contributed rule's `Flag` outcome
//!    surfaces on the emitted L2 row.
//!
//! No database is required — the test pins the *pure* seam
//! (`process_entity_window`); `run_tick` adds only the L1 fetch +
//! L2 bulk insert around the same walker, which is exercised by
//! the existing cleaner unit tests.
//!
//! Requires the multi-thread tokio runtime because
//! [`rubix_tools::cleaner::adapter::ToolAnomalyRule`] bridges the
//! sync `AnomalyRule::apply` into the async tool dispatch with
//! [`tokio::task::block_in_place`].

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_spi::LifecycleState;
use starter_spi::error::Result as SpiResult;
use starter_spi::tool::{Tool, ToolDefinition};

use rubix_agent::registry::collect_anomaly_rule_contributions;
use rubix_tools::cleaner::adapter::build_registry_with_contributions;
use rubix_tools::cleaner::{process_entity_window, QualityTag, Reading};

/// Bundle id used by every fixture in this file. Owns the
/// `com.acme.weather.*` namespace so the contributed rule id
/// (`com.acme.weather.spike`) passes R4 namespace-ownership.
const EXT_ID: &str = "com.acme.weather";

/// Tool id the contributed rule dispatches against. Same dotted
/// descendant of the bundle id; the bundle's
/// `contributes.tools[]` *also* declares this id (it is the
/// extension's own tool) so the manifest is internally
/// consistent, but the adapter resolves by `definition().name`
/// against the supplied tool list — see below for the
/// [`CannedTool`] that stands in for the real tool here.
const TOOL_ID: &str = "com.acme.weather.spike_check";

/// Rule id the contributed rule announces to the cleaner.
const RULE_ID: &str = "com.acme.weather.spike";

/// Canonical fixture manifest. `runtime.kind: builtin` keeps the
/// loader from touching the filesystem beyond the YAML itself —
/// no schema files or binaries need to exist on disk.
const FIXTURE_YAML: &str = r#"v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Acme Weather (gate test)"
runtime: { kind: builtin, crate_name: weather }
contributes:
  anomaly_rules:
    - id: com.acme.weather.spike
      tool_id: com.acme.weather.spike_check
"#;

/// Minimal `Tool` impl that returns a canned JSON response per
/// call. Mirrors the `CannedTool` in
/// `rubix-tools/src/cleaner/adapter.rs` test module — duplicated
/// here so this integration test does not depend on a `#[cfg(test)]`
/// item.
struct CannedTool {
    name: String,
    responses: Mutex<Vec<Value>>,
}

impl std::fmt::Debug for CannedTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CannedTool")
            .field("name", &self.name)
            .finish()
    }
}

#[async_trait]
impl Tool for CannedTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: "gate-test fake".into(),
            input_schema: serde_json::json!({}),
        }
    }

    async fn invoke(&self, _input: Value) -> SpiResult<Value> {
        let mut q = self.responses.lock().unwrap();
        assert!(!q.is_empty(), "CannedTool ran out of canned responses");
        Ok(q.remove(0))
    }
}

fn canned(name: &str, responses: Vec<Value>) -> Arc<dyn Tool> {
    Arc::new(CannedTool {
        name: name.into(),
        responses: Mutex::new(responses),
    })
}

/// Write the fixture bundle into a fresh tempdir and return the
/// dir handle (kept alive so the directory is not deleted while
/// the loader is reading it).
fn write_fixture_bundle() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let bundle = dir.path().join(EXT_ID);
    std::fs::create_dir_all(&bundle).expect("create bundle dir");
    std::fs::write(bundle.join("block.yaml"), FIXTURE_YAML).expect("write block.yaml");
    dir
}

/// Drive the loader end-to-end and return a sealed, Arc'd
/// registry along with the fixture dir handle.
fn load_fixture() -> (Arc<ExtensionRegistry>, tempfile::TempDir) {
    let dir = write_fixture_bundle();
    let records = Loader::scan(dir.path()).validate_all();
    assert_eq!(records.len(), 1, "expected exactly one fixture bundle");
    assert_eq!(
        records[0].state,
        LifecycleState::Validated,
        "fixture manifest failed validation: {:?}",
        records[0].failure,
    );

    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    assert_eq!(outcome.validated, 1);
    assert_eq!(outcome.failed, 0);
    registry.seal();
    (Arc::new(registry), dir)
}

/// Synthetic L1 reading. Picks values that pass every builtin
/// rule (finite, no history → SpikeRule + StuckRule can't fire)
/// so any flag must come from the contributed rule.
fn ordinary_reading() -> Reading {
    Reading {
        tenant_id: "t-acme".into(),
        entity_id: "sensor-1".into(),
        ts_ms: 1_700_000_000_000,
        value: Some(21.5),
        source_quality: 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_validates_through_extension_registry() {
    let (registry, _dir) = load_fixture();
    let rec = registry
        .get_by_id_str(EXT_ID)
        .expect("validated record present");
    assert!(rec.is_validated());
    let manifest = rec.manifest.as_ref().expect("manifest survives commit");
    assert_eq!(manifest.contributes.anomaly_rules.len(), 1);
    assert_eq!(manifest.contributes.anomaly_rules[0].id, RULE_ID);
    assert_eq!(manifest.contributes.anomaly_rules[0].tool_id, TOOL_ID);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_projection_emits_one_contribution_per_manifest_entry() {
    let (registry, _dir) = load_fixture();
    let contributions = collect_anomaly_rule_contributions(Some(&registry));
    assert_eq!(contributions.len(), 1);
    assert_eq!(contributions[0].id, RULE_ID);
    assert_eq!(contributions[0].tool_id, TOOL_ID);
    assert!(contributions[0].priority.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contributed_rule_fires_end_to_end_against_synthetic_window() {
    let (registry, _dir) = load_fixture();
    let contributions = collect_anomaly_rule_contributions(Some(&registry));

    let tool = canned(
        TOOL_ID,
        vec![serde_json::json!({
            "outcome": "flag",
            "quality": "spike",
            "note": "ratio=37x",
        })],
    );

    let rule_registry = build_registry_with_contributions(&[tool], contributions);
    // 3 builtins (NaN → Spike → Stuck) + 1 contributed rule.
    assert_eq!(rule_registry.len(), 4);
    assert_eq!(
        rule_registry.ids().collect::<Vec<_>>(),
        vec!["builtin.nan", "builtin.spike", "builtin.stuck", RULE_ID],
    );

    let row = ordinary_reading();
    let (emitted, dropped) = process_entity_window(&rule_registry, &[], &[row.clone()]);
    assert_eq!(dropped, 0);
    assert_eq!(emitted.len(), 1);
    let l2 = &emitted[0];
    assert_eq!(l2.tenant_id, row.tenant_id);
    assert_eq!(l2.entity_id, row.entity_id);
    assert_eq!(l2.ts_ms, row.ts_ms);
    assert_eq!(l2.value, row.value);
    assert_eq!(l2.quality, QualityTag::Spike);
    assert_eq!(l2.rule_id, Some(RULE_ID));
    assert_eq!(
        l2.tags,
        serde_json::json!({ RULE_ID: "ratio=37x" }),
        "flag note must surface in L2 tags JSONB",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contributed_rule_passes_through_when_tool_says_ok() {
    let (registry, _dir) = load_fixture();
    let contributions = collect_anomaly_rule_contributions(Some(&registry));
    let tool = canned(TOOL_ID, vec![serde_json::json!({ "outcome": "ok" })]);
    let rule_registry = build_registry_with_contributions(&[tool], contributions);

    let row = ordinary_reading();
    let (emitted, dropped) = process_entity_window(&rule_registry, &[], &[row]);
    assert_eq!(dropped, 0);
    assert_eq!(emitted.len(), 1);
    let l2 = &emitted[0];
    assert_eq!(l2.quality, QualityTag::Ok);
    assert!(l2.rule_id.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_nan_rule_short_circuits_contributed_rule() {
    // Confirms the ordering invariant the adapter docs promise:
    // a NaN row tags as `builtin.nan` even when a contributed
    // rule would otherwise flag it, because builtins always run
    // first. The CannedTool's response queue stays untouched —
    // proving the contributed rule never fires.
    let (registry, _dir) = load_fixture();
    let contributions = collect_anomaly_rule_contributions(Some(&registry));
    let tool_inner = Arc::new(CannedTool {
        name: TOOL_ID.into(),
        responses: Mutex::new(vec![serde_json::json!({ "outcome": "drop" })]),
    });
    let rule_registry =
        build_registry_with_contributions(&[tool_inner.clone() as Arc<dyn Tool>], contributions);

    let nan_row = Reading {
        tenant_id: "t-acme".into(),
        entity_id: "sensor-1".into(),
        ts_ms: 1_700_000_000_000,
        value: Some(f64::NAN),
        source_quality: 0,
    };
    let (emitted, _dropped) = process_entity_window(&rule_registry, &[], &[nan_row]);
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].quality, QualityTag::Nan);
    assert_eq!(emitted[0].rule_id, Some("builtin.nan"));
    assert_eq!(
        tool_inner.responses.lock().unwrap().len(),
        1,
        "contributed rule must NOT fire when a builtin short-circuits",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unresolved_tool_id_is_silently_dropped() {
    // Sanity-check the builder's miss path through the manifest
    // entry point: if the host's tool registry has no tool whose
    // `definition().name` matches the contribution's `tool_id`,
    // the rule is dropped (warn-logged) and the cleaner runs
    // with just the builtins.
    let (registry, _dir) = load_fixture();
    let contributions = collect_anomaly_rule_contributions(Some(&registry));
    let rule_registry = build_registry_with_contributions(&[], contributions);
    assert_eq!(rule_registry.len(), 3);
    assert_eq!(
        rule_registry.ids().collect::<Vec<_>>(),
        vec!["builtin.nan", "builtin.spike", "builtin.stuck"],
    );
}

#[test]
fn host_projection_with_no_registry_returns_empty() {
    // The boot path passes `None` for the laptop / `rubix-admin
    // mcp` stdio paths that don't load extensions. The projection
    // must degrade to "no contributed rules" without panicking.
    let contributions = collect_anomaly_rule_contributions(None);
    assert!(contributions.is_empty());
}
