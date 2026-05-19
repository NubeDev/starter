//! SCOPE smoke: "Crash loop is bounded" (R9).
//!
//! A `RestartTracker` configured with `max_restarts = 5` /
//! `within_seconds = 60` accepts five Crash exits, then refuses the
//! sixth with `Failed`. Pure-state-machine assertion — no child
//! spawning needed — because the production policy lives in
//! `RestartTracker` and the `SupervisorTask` is a thin wrapper around it
//! (see `supervisor.rs:276`).
//!
//! Pinning the policy here means a refactor of `RestartTracker` that
//! quietly drops the cap shows up as a `starter-ext-smoke` failure,
//! not three crates away.

use starter_ext_spi::{Backoff, HealthConfig, RestartPolicy, Supervision};
use starter_ext_supervisor::{RestartDecision, RestartTracker};

fn supervision(policy: RestartPolicy, max: u32, within: u32) -> Supervision {
    Supervision {
        restart: policy,
        max_restarts: max,
        within_seconds: within,
        backoff: Backoff::default(),
        health: HealthConfig::default(),
        group: None,
        shutdown_grace_ms: 5_000,
    }
}

#[test]
fn always_policy_hits_intensity_cap_then_transitions_to_failed() {
    let mut t = RestartTracker::from_manifest(&supervision(RestartPolicy::Always, 5, 60));
    use starter_ext_spi::LifecycleState;

    // Use the public `should_restart` surface; the body advances the
    // sliding window internally.
    for i in 0..5 {
        let decision = t.should_restart(starter_ext_supervisor::restart::ExitReason::Crash);
        assert_eq!(
            decision,
            RestartDecision::Restart,
            "attempt {i} (0-indexed) should still be under the cap"
        );
    }
    let cap = t.should_restart(starter_ext_supervisor::restart::ExitReason::Crash);
    assert_eq!(
        cap,
        RestartDecision::Failed,
        "the sixth crash within the window must trip the intensity cap"
    );
    assert_eq!(
        cap.lifecycle(),
        LifecycleState::Failed,
        "Failed decision must map onto the Failed lifecycle state — the \
         admin endpoint reads this directly (SCOPE R9)"
    );

    // And it stays Failed: even if the operator's restart policy would
    // try again, the cap is sticky.
    assert_eq!(
        t.should_restart(starter_ext_supervisor::restart::ExitReason::Crash),
        RestartDecision::Failed,
    );

    // Compile-time check that the supervisor's exported `EventKind` is
    // still in scope — a refactor that hides it from the public API
    // would break the smoke crate's admin observability story.
    fn _kind_is_exported() -> Option<starter_ext_supervisor::EventKind> {
        None
    }
    let _ = _kind_is_exported();
}

#[test]
fn on_crash_clean_exit_stops_immediately() {
    let mut t = RestartTracker::from_manifest(&supervision(RestartPolicy::OnCrash, 5, 60));
    assert_eq!(
        t.should_restart(starter_ext_supervisor::restart::ExitReason::Clean),
        RestartDecision::Stop,
        "a clean exit under on_crash policy is the normal shutdown path, not a crash"
    );
}

#[test]
fn never_policy_stops_on_any_exit() {
    let mut t = RestartTracker::from_manifest(&supervision(RestartPolicy::Never, 5, 60));
    assert_eq!(
        t.should_restart(starter_ext_supervisor::restart::ExitReason::Crash),
        RestartDecision::Stop,
    );
    assert_eq!(
        t.should_restart(starter_ext_supervisor::restart::ExitReason::Clean),
        RestartDecision::Stop,
    );
}
