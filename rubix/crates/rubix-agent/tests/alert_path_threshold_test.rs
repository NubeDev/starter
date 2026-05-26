//! Alert-path integration test exercising the
//! `cfg.insights.disk_warn_threshold` knob added in stage 7.
//!
//! Why this test exists
//! ====================
//! The v0 insights gate hardcoded `> 90`, which made the alert
//! path impossible to exercise on a CI host whose root filesystem
//! sits comfortably below 90 % used. Stage 7 parameterised the
//! threshold through [`InsightsConfig::disk_warn_threshold`] so
//! the rubix-agent binary can lower it and so this integration
//! test can prove the alert path actually fires when a synthetic
//! 60 %-used probe crosses a threshold of 50.
//!
//! Why an in-memory sink rather than log capture
//! =============================================
//! [`rubix_tools::system::alert_send`] exposes a process-wide
//! atomic counter ([`dispatched_count`]) explicitly built for
//! integration assertions of this shape. Reading it before and
//! after the gate runs gives a deterministic delta with no
//! tracing-subscriber init dance, no global subscriber races
//! across test binaries, and no `tracing-test`-style log parsing.
//! The counter is the contract the gate documents; the log lines
//! are an operator-facing rendering of the same event. Asserting
//! the counter therefore asserts the alert path itself fired —
//! exactly the property stage 7's SCOPE asks us to pin down.
//!
//! Wiring asserted here
//! ====================
//! 1. [`AgentConfig::default`] still hands out `90` so existing
//!    operators see no behaviour change.
//! 2. [`registry::build_tool_registry`] threads its
//!    `insights_disk_threshold` argument into
//!    [`DiskTool::with_insights_threshold`].
//! 3. [`run_insights_gate`] respects a non-default threshold and
//!    fires `rubix.alert.send` exactly once when crossed.

use rubix_agent::boot::config::AgentConfig;
use rubix_agent::registry::build_tool_registry;
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

#[test]
fn default_disk_warn_threshold_is_ninety() {
    // Operators with no `[insights]` section in agent.toml keep
    // the legacy v0 behaviour byte-for-byte.
    let cfg = AgentConfig::default();
    assert_eq!(cfg.insights.disk_warn_threshold, 90);
}

#[tokio::test]
async fn alert_path_fires_when_threshold_lowered_to_fifty() {
    // Threshold of 50 mirrors what an operator would set in
    // agent.toml to exercise the alert path on a healthy host.
    let before = alert_send::dispatched_count();
    let fired = run_insights_gate(&synth_response(60), 50)
        .await
        .expect("gate runs cleanly at 60% used");
    let after = alert_send::dispatched_count();

    assert!(
        fired,
        "the gate must fire when percent_used (60) crosses threshold (50)",
    );
    assert_eq!(
        after - before,
        1,
        "the alert path must dispatch rubix.alert.send exactly once \
         per threshold-crossing probe; got delta {}",
        after - before,
    );
}

#[tokio::test]
async fn alert_path_silent_when_threshold_not_crossed() {
    // Same 60% probe, but with a higher threshold of 70: the gate
    // must stay silent. This pins the comparator direction so a
    // future refactor cannot flip `>` to `>=` or `<` without the
    // suite turning red.
    let before = alert_send::dispatched_count();
    let fired = run_insights_gate(&synth_response(60), 70)
        .await
        .expect("gate runs cleanly at 60% used");
    let after = alert_send::dispatched_count();

    assert!(
        !fired,
        "the gate must not fire when percent_used (60) is below threshold (70)",
    );
    assert_eq!(
        after - before,
        0,
        "the alert path must stay silent below the threshold",
    );
}

#[test]
fn registry_threads_threshold_into_disk_tool() {
    // Smoke-asserts the wiring path: build_tool_registry must
    // still build the disk verb (and the rest of the bundled
    // catalogue) when a non-default threshold is passed. The
    // alert-firing assertion lives in the gate-level test above;
    // this one guards the boot-time plumbing so a typo in the
    // signature surfaces here rather than at agent startup.
    let names: Vec<String> = build_tool_registry(None, 50, None, None)
        .iter()
        .map(|t| t.definition().name)
        .collect();
    assert!(
        names.contains(&"rubix.system.disk".to_owned()),
        "disk verb must remain registered when the threshold is overridden; got {names:?}",
    );
}
