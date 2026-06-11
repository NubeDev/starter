//! Dry-run a flow's input + pipeline against the bounded collector, without
//! persisting it or writing to its real output.
//!
//! This is the flow editor's "Test" button: validate the engine config for
//! real (a build error is surfaced here, not on save) and return a bounded
//! sample of what the pipeline produces. It reuses the one-shot
//! [`QueryRunner`](nexus_engine::QueryRunner), which swaps the caller's output
//! for the in-memory collector and enforces the run caps, so a test never runs
//! an unbounded stream or touches a real sink.

use std::time::Duration;

use nexus_engine::{Caps, QueryRunner};
use nexus_spi::dto::flow::DryRunResponse;
use serde_json::Value;

/// The hard ceiling on a dry-run sample. The editor may request fewer rows but
/// never more — a test must stay cheap regardless of what the client asks.
const MAX_SAMPLE_ROWS: u64 = 500;
/// The hard ceiling on a dry-run's serialized sample size.
const MAX_SAMPLE_BYTES: u64 = 2 * 1024 * 1024;
/// Wall-clock budget for a dry run. A streaming input (`http_poll`/`simulator`)
/// never returns EOF, so the deadline is what ends the test once enough sample
/// rows are in (or the row cap trips first).
const MAX_SAMPLE_DURATION: Duration = Duration::from_secs(8);

/// Build the caps for a dry run, clamping the caller's `max_rows` to the hard
/// ceiling. A `None` or over-ceiling request uses the ceiling.
fn caps(requested_rows: Option<u64>) -> Caps {
    let rows = requested_rows
        .filter(|r| *r > 0)
        .map(|r| r.min(MAX_SAMPLE_ROWS))
        .unwrap_or(MAX_SAMPLE_ROWS);
    Caps::new(rows, MAX_SAMPLE_BYTES, MAX_SAMPLE_DURATION)
}

/// Run `input` through `processors` under dry-run caps and shape the outcome
/// into a [`DryRunResponse`]. A build/runtime failure that produced no rows
/// becomes a response with `error` set and empty rows — the editor shows it
/// inline rather than the request failing — so the only `Err` from here is the
/// engine failing to initialise, which the caller maps to a 5xx.
pub async fn run(
    input: Value,
    processors: Vec<Value>,
    requested_rows: Option<u64>,
) -> Result<DryRunResponse, String> {
    let runner = QueryRunner::new()?;
    match runner.run(input, processors, caps(requested_rows)).await {
        Ok(outcome) => Ok(DryRunResponse {
            columns: outcome.columns,
            rows: outcome.rows,
            stats: outcome.stats,
            error: None,
        }),
        Err(message) => Ok(DryRunResponse {
            columns: Vec::new(),
            rows: Vec::new(),
            stats: empty_stats(),
            error: Some(message),
        }),
    }
}

/// Zeroed stats for a failed dry run (no rows, no time recorded).
fn empty_stats() -> nexus_spi::dto::query::QueryStats {
    nexus_spi::dto::query::QueryStats {
        row_count: 0,
        byte_count: 0,
        elapsed_ms: 0,
        truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_clamp_to_ceiling() {
        assert_eq!(caps(Some(10)).max_rows, Some(10));
        assert_eq!(caps(Some(99_999)).max_rows, Some(MAX_SAMPLE_ROWS));
        assert_eq!(caps(None).max_rows, Some(MAX_SAMPLE_ROWS));
        assert_eq!(caps(Some(0)).max_rows, Some(MAX_SAMPLE_ROWS));
    }

    #[tokio::test]
    async fn invalid_input_returns_error_in_response_not_err() {
        // An unknown input type is a build failure; the dry run surfaces it as a
        // populated `error`, not a transport error.
        let res = run(serde_json::json!({ "type": "no_such_input" }), vec![], None)
            .await
            .expect("engine init ok");
        assert!(res.error.is_some(), "build failure should be reported inline");
        assert!(res.rows.is_empty());
    }

    #[tokio::test]
    async fn simulator_dry_run_yields_bounded_sample() {
        // A simulator input emits immediately and never EOFs; the row cap stops
        // it, so the sample is bounded and non-empty.
        let input = serde_json::json!({
            "type": "simulator",
            "profile": "hvac",
            "interval": "1s",
            "device_id": "test-1",
        });
        let processors = vec![serde_json::json!({ "type": "json_to_arrow" })];
        let res = run(input, processors, Some(5)).await.expect("engine init ok");
        assert!(res.error.is_none(), "valid flow should not error: {:?}", res.error);
        assert!(!res.rows.is_empty(), "simulator should produce sample rows");
        assert!(res.rows.len() as u64 <= 5, "sample respects the requested cap");
    }
}
