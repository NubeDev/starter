//! `rubix.cleaner.tick` — invoke one cleaner pass.
//!
//! Wraps [`super::tick::run_tick`] so the cleaner can be driven by
//! the same `starter.flow.tool-call` node any other tool uses. The
//! input is a `{ from_ts_ms, to_ts_ms, history_lookback_ms? }`
//! payload; the output is the [`TickStats`] returned by
//! `run_tick`, serialised verbatim.
//!
//! ## `tick_epoch_ms` auto-injection
//!
//! The agent's tool-call seed adapter auto-injects
//! `tool_input.tick_epoch_ms` from wall-clock on every fire (same
//! path the synth producer uses). When `from_ts_ms` / `to_ts_ms`
//! are absent on the wire we derive them from `tick_epoch_ms`:
//!
//! - `to_ts_ms` ← `tick_epoch_ms`
//! - `from_ts_ms` ← `tick_epoch_ms - window_ms` (default 60_000)
//!
//! so the bundled `com.rubix.cleaner` flow's `tool_input: {}` Just
//! Works — every tick cleans the last 60 seconds of L1.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::WarehouseClient;

use super::registry::RuleRegistry;
use super::tick::{run_tick, TickParams};

/// Concrete `Tool` impl for `rubix.cleaner.tick`.
pub struct CleanerTickTool {
    client: WarehouseClient,
    registry: RuleRegistry,
}

impl std::fmt::Debug for CleanerTickTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanerTickTool")
            .field("rule_count", &self.registry.len())
            .finish()
    }
}

impl CleanerTickTool {
    /// New tool bound to a warehouse pool and a rule registry. The
    /// agent typically passes `RuleRegistry::builtin()`; tests may
    /// pass an empty registry to exercise the pass-through path.
    pub fn new(client: WarehouseClient, registry: RuleRegistry) -> Self {
        Self { client, registry }
    }
}

#[derive(Debug, Deserialize)]
struct TickRequest {
    #[serde(default)]
    from_ts_ms: Option<i64>,
    #[serde(default)]
    to_ts_ms: Option<i64>,
    #[serde(default)]
    history_lookback_ms: Option<i64>,
    /// Wall-clock epoch ms auto-injected by the host seed adapter.
    /// Used to derive `from`/`to` when they are absent.
    #[serde(default)]
    tick_epoch_ms: Option<i64>,
    /// Sliding window length when `from`/`to` are derived from
    /// `tick_epoch_ms`. Defaults to 60_000 (one minute).
    #[serde(default)]
    window_ms: Option<i64>,
}

/// Default sliding-window length when neither `from_ts_ms` nor
/// `to_ts_ms` are present on the wire. Matches the bundled flow's
/// 60s schedule so successive ticks tile exactly.
pub const DEFAULT_WINDOW_MS: i64 = 60_000;

/// Default history lookback when the caller does not override.
/// Matches [`TickParams::new`].
pub const DEFAULT_HISTORY_LOOKBACK_MS: i64 = 30 * 60 * 1000;

#[async_trait]
impl Tool for CleanerTickTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.cleaner.tick".to_owned(),
            description: "Run one cleaner pass over a window of `samples`, applying registered \
                          anomaly rules and writing the result into `samples_l2`."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from_ts_ms": { "type": "integer" },
                    "to_ts_ms":   { "type": "integer" },
                    "history_lookback_ms": { "type": "integer" },
                    "tick_epoch_ms": { "type": "integer" },
                    "window_ms": { "type": "integer" }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: TickRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("TickRequest: {e}"),
        })?;
        let params = resolve_params(&req)?;
        let stats = run_tick(&self.client, &self.registry, &params)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
        serde_json::to_value(&stats).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Resolve a [`TickParams`] from the wire request. Exposed so the
/// unit tests don't need a warehouse pool.
fn resolve_params(req: &TickRequest) -> Result<TickParams> {
    let window_ms = req.window_ms.unwrap_or(DEFAULT_WINDOW_MS);
    let (from_ts_ms, to_ts_ms) = match (req.from_ts_ms, req.to_ts_ms, req.tick_epoch_ms) {
        (Some(f), Some(t), _) => (f, t),
        (Some(f), None, Some(tick)) => (f, tick),
        (None, Some(t), _) => (t.saturating_sub(window_ms), t),
        (None, None, Some(tick)) => (tick.saturating_sub(window_ms), tick),
        _ => {
            return Err(Error::Invalid {
                message: "cleaner.tick: need at least one of {from_ts_ms+to_ts_ms, tick_epoch_ms}"
                    .into(),
            })
        }
    };
    if to_ts_ms < from_ts_ms {
        return Err(Error::Invalid {
            message: format!("cleaner.tick: to_ts_ms ({to_ts_ms}) < from_ts_ms ({from_ts_ms})"),
        });
    }
    Ok(TickParams {
        from_ts_ms,
        to_ts_ms,
        history_lookback_ms: req
            .history_lookback_ms
            .unwrap_or(DEFAULT_HISTORY_LOOKBACK_MS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(
        from: Option<i64>,
        to: Option<i64>,
        tick: Option<i64>,
        window: Option<i64>,
    ) -> TickRequest {
        TickRequest {
            from_ts_ms: from,
            to_ts_ms: to,
            history_lookback_ms: None,
            tick_epoch_ms: tick,
            window_ms: window,
        }
    }

    #[test]
    fn explicit_from_to_passes_through() {
        let p = resolve_params(&req(Some(100), Some(200), None, None)).unwrap();
        assert_eq!(p.from_ts_ms, 100);
        assert_eq!(p.to_ts_ms, 200);
        assert_eq!(p.history_lookback_ms, DEFAULT_HISTORY_LOOKBACK_MS);
    }

    #[test]
    fn tick_epoch_ms_alone_derives_window() {
        let p = resolve_params(&req(None, None, Some(1_000_000), None)).unwrap();
        assert_eq!(p.from_ts_ms, 1_000_000 - DEFAULT_WINDOW_MS);
        assert_eq!(p.to_ts_ms, 1_000_000);
    }

    #[test]
    fn custom_window_ms_honored() {
        let p = resolve_params(&req(None, None, Some(10_000), Some(2_500))).unwrap();
        assert_eq!(p.from_ts_ms, 10_000 - 2_500);
        assert_eq!(p.to_ts_ms, 10_000);
    }

    #[test]
    fn neither_from_to_nor_tick_is_an_error() {
        let err = resolve_params(&req(None, None, None, None)).unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[test]
    fn reversed_range_is_rejected() {
        let err = resolve_params(&req(Some(200), Some(100), None, None)).unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[test]
    fn only_to_uses_window_to_derive_from() {
        let p = resolve_params(&req(None, Some(5_000), None, Some(1_000))).unwrap();
        assert_eq!(p.from_ts_ms, 4_000);
        assert_eq!(p.to_ts_ms, 5_000);
    }
}
