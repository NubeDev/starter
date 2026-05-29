//! [`ToolAnomalyRule`] — wrap a `starter_spi::Tool` dispatch into
//! an [`AnomalyRule`].
//!
//! This is the adapter the host uses to plug
//! `contributes.anomaly_rules[]` entries into the cleaner's
//! [`RuleRegistry`]. Each entry names a `tool_id`; the host
//! resolves it against its tool registry, builds one
//! [`ToolAnomalyRule`], and appends it to the registry after the
//! builtins (the registry's "first non-Ok wins" rule then makes
//! ordering operator-controllable).
//!
//! ## Wire shape
//!
//! The adapter invokes the tool with the JSON payload:
//!
//! ```json
//! {
//!   "row": { "tenant_id": "...", "entity_id": "...", "ts_ms": 0,
//!            "value": 0.0, "source_quality": 0 },
//!   "window_tail": [ /* preceding readings, oldest-first */ ]
//! }
//! ```
//!
//! and expects one of these responses:
//!
//! ```json
//! { "outcome": "ok" }
//! { "outcome": "flag", "quality": "spike", "note": "ratio=37x" }
//! { "outcome": "drop" }
//! ```
//!
//! Errors / shape mismatches degrade to [`RuleOutcome::Ok`] (the
//! safe default — a misbehaving rule must not silently flag rows)
//! with a warning logged under target `rubix.cleaner.adapter`.
//!
//! ## Sync-trait bridge
//!
//! [`AnomalyRule::apply`] is sync + infallible per the trait's
//! contract (rule walker is hot, per-row). [`starter_spi::Tool`] is
//! async + fallible. We bridge inside [`AnomalyRule::apply`] with
//! [`tokio::task::block_in_place`] + the current
//! [`tokio::runtime::Handle`]. **The adapter therefore requires a
//! multi-thread tokio runtime.** Calling it from a single-thread
//! runtime will panic at the `block_in_place` boundary — the
//! cleaner tick (`run_tick`) is the canonical caller and the agent
//! runs on a multi-thread runtime by default.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use starter_spi::tool::Tool;

use super::rule::{AnomalyRule, QualityTag, Reading, RuleOutcome, WindowSlice};

/// Adapter wrapping a `Tool` dispatch as an `AnomalyRule`.
pub struct ToolAnomalyRule {
    /// Leaked at construction so the trait's `id() -> &'static
    /// str` contract still holds for dynamic ids that originate
    /// in an extension manifest. Boot-time leak only; size is
    /// bounded by the manifest, not by run-time invocations.
    id: &'static str,
    tool: Arc<dyn Tool>,
}

impl std::fmt::Debug for ToolAnomalyRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolAnomalyRule")
            .field("id", &self.id)
            .field("tool", &self.tool.definition().name)
            .finish()
    }
}

impl ToolAnomalyRule {
    /// New adapter for an extension-contributed rule.
    ///
    /// `id` is the manifest's `contributes.anomaly_rules[].id`. The
    /// string is [`Box::leak`]ed so the trait's `&'static str`
    /// contract holds; the host calls this once per manifest entry
    /// at boot so the leak is bounded.
    pub fn new(id: impl Into<String>, tool: Arc<dyn Tool>) -> Self {
        Self {
            id: Box::leak(id.into().into_boxed_str()),
            tool,
        }
    }
}

/// Wire shape the adapter expects back from the tool.
#[derive(Debug, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum ToolOutcome {
    Ok,
    Flag {
        quality: QualityTag,
        #[serde(default)]
        note: Option<String>,
    },
    Drop,
}

impl AnomalyRule for ToolAnomalyRule {
    fn id(&self) -> &'static str {
        self.id
    }

    fn apply(&self, row: &Reading, window: WindowSlice<'_>) -> RuleOutcome {
        let payload = serde_json::json!({
            "row": row,
            "window_tail": window.history,
        });
        let outcome = invoke_blocking(&*self.tool, payload);
        match outcome {
            Ok(ToolOutcome::Ok) => RuleOutcome::Ok,
            Ok(ToolOutcome::Flag { quality, note }) => RuleOutcome::Flag { quality, note },
            Ok(ToolOutcome::Drop) => RuleOutcome::Drop,
            Err(message) => {
                tracing::warn!(
                    target: "rubix.cleaner.adapter",
                    rule_id = %self.id,
                    error = %message,
                    "tool-backed rule fell back to Ok",
                );
                RuleOutcome::Ok
            }
        }
    }
}

/// Drive `tool.invoke(payload)` to completion from a synchronous
/// context. Returns the parsed [`ToolOutcome`] or a human message
/// describing the failure (used for the warn log in
/// [`AnomalyRule::apply`]).
fn invoke_blocking(tool: &dyn Tool, payload: Value) -> Result<ToolOutcome, String> {
    let fut = tool.invoke(payload);
    let raw = tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
        .map_err(|e| format!("tool invoke: {e}"))?;
    serde_json::from_value::<ToolOutcome>(raw).map_err(|e| format!("tool response shape: {e}"))
}

/// One manifest entry to be wired as a [`ToolAnomalyRule`].
///
/// Decoupled from `starter_ext_spi::ContributeAnomalyRule` so this
/// crate doesn't need to depend on the extensions SPI — the host
/// (rubix-agent) walks the registry, projects each entry into this
/// shape, and hands it to [`build_registry_with_contributions`].
#[derive(Debug, Clone)]
pub struct ContributedRule {
    /// Stable rule id (already validated against the extension's
    /// namespace + `builtin.*` reservation upstream).
    pub id: String,
    /// Tool id the adapter dispatches against. Resolved against
    /// the tool list at build time.
    pub tool_id: String,
    /// Ordering hint. Lower runs earlier; ties preserve declaration
    /// order. `None` sorts last (declared rules opt in to ordering).
    pub priority: Option<i32>,
}

/// Build a [`RuleRegistry`] seeded with the builtins and extended
/// with one [`ToolAnomalyRule`] per contributed entry that resolves
/// against `tools`.
///
/// Ordering: builtins first (their canonical NaN → Spike → Stuck
/// order), then contributed rules sorted by:
///
/// 1. `priority` ascending (`None` last)
/// 2. declaration order on ties
///
/// Tool resolution: each `tool_id` is looked up in `tools` by its
/// `definition().name`. A miss is logged at `warn` under target
/// `rubix.cleaner.adapter` and the rule is **silently dropped** —
/// the cleaner keeps running, the missing rule simply doesn't
/// participate. A second tick will re-resolve if `tools` is rebuilt.
pub fn build_registry_with_contributions(
    tools: &[Arc<dyn Tool>],
    contributions: impl IntoIterator<Item = ContributedRule>,
) -> super::RuleRegistry {
    use super::RuleRegistry;
    let by_name: std::collections::HashMap<String, Arc<dyn Tool>> = tools
        .iter()
        .map(|t| (t.definition().name, t.clone()))
        .collect();

    let mut entries: Vec<(usize, ContributedRule)> = contributions.into_iter().enumerate().collect();
    // Sort by (priority asc with None last, declaration index asc).
    entries.sort_by(|(ai, a), (bi, b)| {
        let pa: (u8, i32) = match a.priority {
            Some(p) => (0, p),
            None => (1, 0),
        };
        let pb: (u8, i32) = match b.priority {
            Some(p) => (0, p),
            None => (1, 0),
        };
        pa.cmp(&pb).then_with(|| ai.cmp(bi))
    });

    let mut registry = RuleRegistry::builtin();
    for (_, c) in entries {
        match by_name.get(&c.tool_id) {
            Some(tool) => {
                registry = registry.add(Arc::new(ToolAnomalyRule::new(c.id, tool.clone())));
            }
            None => {
                tracing::warn!(
                    target: "rubix.cleaner.adapter",
                    rule_id = %c.id,
                    tool_id = %c.tool_id,
                    "contributed anomaly rule references an unknown tool; rule dropped",
                );
            }
        }
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use starter_spi::error::Result as SpiResult;
    use starter_spi::tool::ToolDefinition;
    use std::sync::Mutex;

    /// Test tool that returns a canned JSON response per call.
    struct CannedTool {
        responses: Mutex<Vec<Value>>,
    }

    impl std::fmt::Debug for CannedTool {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("CannedTool").finish()
        }
    }

    #[async_trait]
    impl Tool for CannedTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "test.canned".into(),
                description: "test".into(),
                input_schema: serde_json::json!({}),
            }
        }
        async fn invoke(&self, _input: Value) -> SpiResult<Value> {
            Ok(self.responses.lock().unwrap().remove(0))
        }
    }

    fn r() -> Reading {
        Reading {
            tenant_id: "t".into(),
            entity_id: "e".into(),
            ts_ms: 1,
            value: Some(1.0),
            source_quality: 0,
        }
    }

    fn tool(resp: Value) -> Arc<dyn Tool> {
        Arc::new(CannedTool {
            responses: Mutex::new(vec![resp]),
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ok_outcome_passes_through() {
        let rule = ToolAnomalyRule::new(
            "com.acme.weather.spike",
            tool(serde_json::json!({"outcome": "ok"})),
        );
        let out = rule.apply(&r(), WindowSlice::new(&[]));
        assert!(matches!(out, RuleOutcome::Ok));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn flag_outcome_carries_quality_and_note() {
        let rule = ToolAnomalyRule::new(
            "com.acme.x",
            tool(serde_json::json!({"outcome": "flag", "quality": "spike", "note": "ratio=37x"})),
        );
        match rule.apply(&r(), WindowSlice::new(&[])) {
            RuleOutcome::Flag { quality, note } => {
                assert_eq!(quality, QualityTag::Spike);
                assert_eq!(note.as_deref(), Some("ratio=37x"));
            }
            other => panic!("expected Flag, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn drop_outcome_propagates() {
        let rule = ToolAnomalyRule::new("com.acme.x", tool(serde_json::json!({"outcome": "drop"})));
        assert!(matches!(
            rule.apply(&r(), WindowSlice::new(&[])),
            RuleOutcome::Drop
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn malformed_response_degrades_to_ok() {
        let rule =
            ToolAnomalyRule::new("com.acme.x", tool(serde_json::json!({"outcome": "bogus"})));
        // Misbehaving tools must NOT silently flag rows.
        assert!(matches!(
            rule.apply(&r(), WindowSlice::new(&[])),
            RuleOutcome::Ok
        ));
    }

    fn named_tool(name: &str) -> Arc<dyn Tool> {
        struct N(String);
        impl std::fmt::Debug for N {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("N").finish()
            }
        }
        #[async_trait::async_trait]
        impl Tool for N {
            fn definition(&self) -> ToolDefinition {
                ToolDefinition {
                    name: self.0.clone(),
                    description: String::new(),
                    input_schema: serde_json::json!({}),
                }
            }
            async fn invoke(&self, _: Value) -> SpiResult<Value> {
                Ok(serde_json::json!({"outcome": "ok"}))
            }
        }
        Arc::new(N(name.into()))
    }

    #[test]
    fn builder_starts_with_builtins() {
        let reg = build_registry_with_contributions(&[], std::iter::empty());
        assert_eq!(reg.len(), 3);
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(ids, vec!["builtin.nan", "builtin.spike", "builtin.stuck"]);
    }

    #[test]
    fn builder_appends_contributions_after_builtins() {
        let tools = vec![named_tool("com.acme.x"), named_tool("com.acme.y")];
        let reg = build_registry_with_contributions(
            &tools,
            vec![
                ContributedRule {
                    id: "com.acme.x.rule".into(),
                    tool_id: "com.acme.x".into(),
                    priority: None,
                },
                ContributedRule {
                    id: "com.acme.y.rule".into(),
                    tool_id: "com.acme.y".into(),
                    priority: None,
                },
            ],
        );
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(
            ids,
            vec![
                "builtin.nan",
                "builtin.spike",
                "builtin.stuck",
                "com.acme.x.rule",
                "com.acme.y.rule",
            ]
        );
    }

    #[test]
    fn builder_sorts_by_priority_then_declaration_order() {
        let tools = vec![
            named_tool("t.a"),
            named_tool("t.b"),
            named_tool("t.c"),
            named_tool("t.d"),
        ];
        let reg = build_registry_with_contributions(
            &tools,
            vec![
                ContributedRule {
                    id: "r.a".into(),
                    tool_id: "t.a".into(),
                    priority: None,
                },
                ContributedRule {
                    id: "r.b".into(),
                    tool_id: "t.b".into(),
                    priority: Some(10),
                },
                ContributedRule {
                    id: "r.c".into(),
                    tool_id: "t.c".into(),
                    priority: Some(-5),
                },
                ContributedRule {
                    id: "r.d".into(),
                    tool_id: "t.d".into(),
                    priority: Some(10),
                },
            ],
        );
        let ids: Vec<_> = reg.ids().skip(3).collect(); // skip builtins
                                                       // r.c (-5) → r.b (10) → r.d (10, decl ties to r.b) → r.a (None last)
        assert_eq!(ids, vec!["r.c", "r.b", "r.d", "r.a"]);
    }

    #[test]
    fn builder_drops_unresolved_tool_ids() {
        let tools = vec![named_tool("known")];
        let reg = build_registry_with_contributions(
            &tools,
            vec![
                ContributedRule {
                    id: "good".into(),
                    tool_id: "known".into(),
                    priority: None,
                },
                ContributedRule {
                    id: "bad".into(),
                    tool_id: "missing".into(),
                    priority: None,
                },
            ],
        );
        let ids: Vec<_> = reg.ids().collect();
        // builtins + `good`, no `bad`.
        assert_eq!(ids.len(), 4);
        assert!(ids.contains(&"good"));
        assert!(!ids.contains(&"bad"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn id_passes_through_to_trait() {
        let rule = ToolAnomalyRule::new(
            "com.acme.weather.spike",
            tool(serde_json::json!({"outcome": "ok"})),
        );
        assert_eq!(rule.id(), "com.acme.weather.spike");
    }
}
