//! Per-flow failure policy, parsed from the input/output node config blobs.
//!
//! The contract (roadmap §6, RW-08 scope): a sink write error retries with capped
//! backoff, then the flow acts on its `on_error` policy; a source read error
//! retries with capped backoff or halts per `source_on_error`. Both policies are
//! additive keys on the existing opaque node config — no wire-contract change — so
//! an older flow that sets neither gets the documented defaults (`halt` on the
//! sink, `retry_backoff` on the source).
//!
//! See docs/scope is not referenced from code; the present-tense behaviour is
//! documented on each item here.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

/// Default cap on retry attempts for a transient read/write error before the
/// policy's terminal action (halt/drop/dlq) takes over. Bounded so a permanently
/// broken sink cannot retry forever.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// First backoff delay; each subsequent attempt doubles up to [`MAX_BACKOFF`].
pub const DEFAULT_BASE_BACKOFF_MS: u64 = 100;

/// Backoff ceiling so an exponential schedule never parks a flow for minutes.
pub const MAX_BACKOFF: Duration = Duration::from_secs(10);

/// What the sink wrapper does once retries are exhausted on a write error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkOnError {
    /// Surface the error and stop the run (default). The in-flight batch is not
    /// silently dropped — the flow enters its `last_error` state.
    Halt,
    /// Drop the failed batch, count it, and keep running. Lossy by choice.
    Drop,
    /// Route the failed batch to the dead-letter Parquet writer, then keep
    /// running. No silent loss: the rows land in the configured dlq directory.
    Dlq,
}

/// What the source wrapper does on a read error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOnError {
    /// Retry with capped backoff, then halt the run if still failing (default).
    RetryBackoff,
    /// Stop the run on the first read error.
    Halt,
}

/// Capped exponential backoff schedule shared by the source and sink wrappers.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    /// Maximum attempts before the terminal action; `0` means a single try.
    pub max_attempts: u32,
    /// First delay; each retry doubles it up to [`MAX_BACKOFF`].
    pub base: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            base: Duration::from_millis(DEFAULT_BASE_BACKOFF_MS),
        }
    }
}

impl Backoff {
    /// The delay before retry `attempt` (1-based): `base * 2^(attempt-1)`, capped.
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(20);
        let scaled = self.base.saturating_mul(1u32 << shift.min(20));
        scaled.min(MAX_BACKOFF)
    }
}

/// The sink-side policy plus its dead-letter writer config (present only when
/// `on_error` is `dlq`).
#[derive(Debug, Clone)]
pub struct SinkPolicy {
    /// Terminal action once retries are exhausted.
    pub on_error: SinkOnError,
    /// Retry schedule for a transient write error.
    pub backoff: Backoff,
    /// The `datasource`-sink config the dlq path writes failed batches through;
    /// always a `file` (Parquet) kind. `None` unless `on_error` is `dlq`.
    pub dlq: Option<Value>,
}

/// The source-side policy.
#[derive(Debug, Clone)]
pub struct SourcePolicy {
    /// Action on a read error.
    pub on_error: SourceOnError,
    /// Retry schedule for a transient read error.
    pub backoff: Backoff,
}

#[derive(Deserialize, Default)]
struct RawBackoff {
    max_attempts: Option<u32>,
    base_backoff_ms: Option<u64>,
}

impl RawBackoff {
    fn into_backoff(self) -> Backoff {
        Backoff {
            max_attempts: self.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS),
            base: Duration::from_millis(
                self.base_backoff_ms.unwrap_or(DEFAULT_BASE_BACKOFF_MS).max(1),
            ),
        }
    }
}

/// Parse the sink policy from the flow's output node config. Unknown/missing keys
/// fall back to `halt` with the default backoff — the safe, non-lossy default.
pub fn sink_policy(output: &Value) -> SinkPolicy {
    let on_error = match output.get("on_error").and_then(Value::as_str) {
        Some("drop") => SinkOnError::Drop,
        Some("dlq") => SinkOnError::Dlq,
        _ => SinkOnError::Halt,
    };
    let backoff = raw_backoff(output).into_backoff();
    let dlq = match on_error {
        SinkOnError::Dlq => Some(dlq_config(output)),
        _ => None,
    };
    SinkPolicy {
        on_error,
        backoff,
        dlq,
    }
}

/// Parse the source policy from the flow's input node config. The default is
/// `retry_backoff`, matching roadmap §6.
pub fn source_policy(input: &Value) -> SourcePolicy {
    let on_error = match input.get("source_on_error").and_then(Value::as_str) {
        Some("halt") => SourceOnError::Halt,
        _ => SourceOnError::RetryBackoff,
    };
    SourcePolicy {
        on_error,
        backoff: raw_backoff(input).into_backoff(),
    }
}

fn raw_backoff(node: &Value) -> RawBackoff {
    node.get("retry")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Build the dead-letter `datasource` file-sink config from the output node's
/// `dlq` block, defaulting the directory and prefix so a bare `on_error: dlq`
/// still has somewhere to land failed batches.
fn dlq_config(output: &Value) -> Value {
    let block = output.get("dlq");
    let dir = block
        .and_then(|b| b.get("dir"))
        .and_then(Value::as_str)
        .unwrap_or("./dead-letter")
        .to_string();
    let prefix = block
        .and_then(|b| b.get("prefix"))
        .and_then(Value::as_str)
        .unwrap_or("dlq")
        .to_string();
    serde_json::json!({
        "type": "datasource",
        "kind": "file",
        "dir": dir,
        "prefix": prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sink_defaults_to_halt() {
        let p = sink_policy(&json!({ "type": "drop" }));
        assert_eq!(p.on_error, SinkOnError::Halt);
        assert!(p.dlq.is_none());
        assert_eq!(p.backoff.max_attempts, DEFAULT_MAX_ATTEMPTS);
    }

    #[test]
    fn sink_drop_and_dlq_parse() {
        assert_eq!(
            sink_policy(&json!({ "on_error": "drop" })).on_error,
            SinkOnError::Drop
        );
        let dlq = sink_policy(&json!({
            "on_error": "dlq",
            "dlq": { "dir": "/tmp/dl", "prefix": "x" }
        }));
        assert_eq!(dlq.on_error, SinkOnError::Dlq);
        let cfg = dlq.dlq.expect("dlq config present");
        assert_eq!(cfg["kind"], "file");
        assert_eq!(cfg["dir"], "/tmp/dl");
        assert_eq!(cfg["prefix"], "x");
    }

    #[test]
    fn source_defaults_to_retry_backoff() {
        assert_eq!(
            source_policy(&json!({ "type": "memory" })).on_error,
            SourceOnError::RetryBackoff
        );
        assert_eq!(
            source_policy(&json!({ "source_on_error": "halt" })).on_error,
            SourceOnError::Halt
        );
    }

    #[test]
    fn retry_block_overrides_defaults() {
        let p = sink_policy(&json!({
            "retry": { "max_attempts": 2, "base_backoff_ms": 50 }
        }));
        assert_eq!(p.backoff.max_attempts, 2);
        assert_eq!(p.backoff.base, Duration::from_millis(50));
    }

    #[test]
    fn backoff_is_capped_and_doubles() {
        let b = Backoff {
            max_attempts: 10,
            base: Duration::from_millis(100),
        };
        assert_eq!(b.delay_for(1), Duration::from_millis(100));
        assert_eq!(b.delay_for(2), Duration::from_millis(200));
        assert_eq!(b.delay_for(3), Duration::from_millis(400));
        assert_eq!(b.delay_for(50), MAX_BACKOFF);
    }
}
