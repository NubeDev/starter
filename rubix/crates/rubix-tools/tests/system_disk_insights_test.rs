//! Integration test for the v0 hardcoded insights gate wired into
//! `rubix.system.disk`'s post-dispatch hook.
//!
//! Asserts the exit-signal contract:
//!   - a synthetic response with `percent_used = 95` fires
//!     `rubix.alert.send` exactly once,
//!   - a synthetic response with `percent_used = 50` fires it zero
//!     times.
//!
//! The gate is exercised directly through
//! [`rubix_tools::system::disk::run_insights_gate`] (rather than the
//! full `Tool::invoke` surface) because the real probe reads the
//! host filesystem and cannot be steered to a precise threshold —
//! the gate is the unit under test, not the probe. The dispatch
//! counter is process-wide so the assertion is a delta, not an
//! absolute.

use rubix_spi::dto::system::disk::DiskUsageResponse;
use rubix_tools::system::alert_send;
use rubix_tools::system::disk::run_insights_gate;
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};

fn synth_response(percent_used: u8) -> DiskUsageResponse {
    DiskUsageResponse {
        summary: Diagnostic::new(
            MessageKey::parse("rubix.system.disk.warn").expect("hard-coded key parses"),
        )
        .with_param("percent", DiagnosticParam::I64(i64::from(percent_used)))
        .with_param("free", DiagnosticParam::I64(100))
        .with_param("at", DiagnosticParam::Timestamp(1_700_000_000_000)),
        mount: "/".to_owned(),
        total_bytes: 1_000_000_000,
        free_bytes: 100,
        percent_used,
        probed_at_ms: 1_700_000_000_000,
    }
}

#[tokio::test]
async fn insights_gate_fires_alert_when_percent_above_threshold() {
    let before = alert_send::dispatched_count();
    let fired = run_insights_gate(&synth_response(95))
        .await
        .expect("gate runs cleanly at 95% used");
    let after = alert_send::dispatched_count();
    assert!(fired, "the v0 rule must fire above 90%");
    assert_eq!(
        after - before,
        1,
        "the gate must dispatch rubix.alert.send exactly once at 95%",
    );
}

#[tokio::test]
async fn insights_gate_is_silent_when_percent_below_threshold() {
    let before = alert_send::dispatched_count();
    let fired = run_insights_gate(&synth_response(50))
        .await
        .expect("gate runs cleanly at 50% used");
    let after = alert_send::dispatched_count();
    assert!(!fired, "the v0 rule must not fire at 50%");
    assert_eq!(
        after - before,
        0,
        "the gate must dispatch rubix.alert.send zero times at 50%",
    );
}
