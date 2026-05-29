//! Issue derivation from the supervisor's owned diagnostics.
//!
//! The supervisor already captures every live-process failure source:
//!
//! - the bounded [`EventRing`](crate::event_ring::EventRing) (crashes,
//!   restart-cap exhaustion, missed health pings, per-call capability
//!   violations), and
//! - the monotone [`CapabilityViolationCounter`](crate::capability::CapabilityViolationCounter).
//!
//! [`derive_issues`] folds those into the workspace-wide
//! [`ExtensionIssue`] read-model so the `GET /extensions/<id>/issues`
//! handler in `starter-ext-server` can merge them with the record-level
//! issues from `starter-ext-host` without knowing the supervisor's
//! internal event vocabulary.
//!
//! The function is pure — `(events, violations) -> issues` — so the
//! derivation is unit-testable without spawning a child process.
//! [`SupervisorHandle::issues`](crate::SupervisorHandle::issues) is the
//! thin wrapper that feeds it the live ring snapshot + counter.

use std::time::SystemTime;

use starter_ext_spi::{ExtensionIssue, IssueCode, IssueSource, Severity};

use crate::event_ring::{Event as RingEvent, EventKind};

/// Fold a ring snapshot + capability-violation counter into the
/// consolidated issue list. Order follows the input ring (oldest first);
/// the HTTP layer re-sorts by `at` descending after merging with the
/// record-level issues.
///
/// Capability violations are emitted per ring event (carrying the refused
/// `method` and the event `seq`). If the monotone counter records
/// violations the ring no longer holds — because they were evicted once
/// the ring filled — a single aggregate [`IssueCode::CapabilityViolation`]
/// issue is appended so the counter is never silently dropped.
pub fn derive_issues(events: &[RingEvent], capability_violations: u64) -> Vec<ExtensionIssue> {
    let mut issues = Vec::new();
    let mut cap_events_seen: u64 = 0;

    for event in events {
        match &event.kind {
            EventKind::Crashed { reason } => issues.push(ExtensionIssue {
                code: IssueCode::Crashed,
                severity: Severity::Error,
                at: event.at,
                detail: reason.clone(),
                source: IssueSource::Supervisor,
                seq: Some(event.seq),
            }),
            EventKind::RestartCapExceeded { count } => issues.push(ExtensionIssue {
                code: IssueCode::RestartCapExceeded,
                severity: Severity::Fatal,
                at: event.at,
                detail: format!("restart intensity cap exceeded after {count} restarts"),
                source: IssueSource::Supervisor,
                seq: Some(event.seq),
            }),
            EventKind::HealthTimeout => issues.push(ExtensionIssue {
                code: IssueCode::HealthTimeout,
                severity: Severity::Error,
                at: event.at,
                detail: "missed health ping; child treated as crashed".to_string(),
                source: IssueSource::Health,
                seq: Some(event.seq),
            }),
            EventKind::CapabilityViolation { method } => {
                cap_events_seen += 1;
                issues.push(ExtensionIssue {
                    code: IssueCode::CapabilityViolation,
                    severity: Severity::Warning,
                    at: event.at,
                    detail: method.clone(),
                    source: IssueSource::Capability,
                    seq: Some(event.seq),
                });
            }
            // State transitions, spawns, clean exits, restart scheduling,
            // and stderr lines are not themselves issues — they are the
            // healthy / informational backbone of the event ring.
            EventKind::StateTransition { .. }
            | EventKind::Spawned { .. }
            | EventKind::ExitedClean { .. }
            | EventKind::RestartScheduled { .. }
            | EventKind::Stderr { .. } => {}
        }
    }

    // The counter is monotone and survives ring eviction; if it counted
    // more violations than the ring still holds, surface the remainder as
    // one aggregate issue so the diagnostic is never lost.
    if capability_violations > cap_events_seen {
        let lost = capability_violations - cap_events_seen;
        issues.push(ExtensionIssue {
            code: IssueCode::CapabilityViolation,
            severity: Severity::Warning,
            at: SystemTime::now(),
            detail: format!(
                "{lost} capability violation(s) recorded but evicted from the event ring"
            ),
            source: IssueSource::Capability,
            seq: None,
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_ring::EventRing;

    fn snapshot(push: impl FnOnce(&EventRing)) -> Vec<RingEvent> {
        let ring = EventRing::new();
        push(&ring);
        ring.snapshot()
    }

    #[test]
    fn crash_event_yields_crashed_issue() {
        let events = snapshot(|r| {
            r.push(EventKind::Spawned { pid: 42 });
            r.push(EventKind::Crashed {
                reason: "non-zero exit code Some(1)".into(),
            });
        });
        let issues = derive_issues(&events, 0);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::Crashed);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].source, IssueSource::Supervisor);
        assert_eq!(issues[0].seq, Some(1));
        assert!(issues[0].detail.contains("non-zero"));
    }

    #[test]
    fn capability_violation_counter_yields_issue() {
        // Counter records a violation but the ring holds no
        // CapabilityViolation event (e.g. it was evicted) — the aggregate
        // path must still surface it.
        let issues = derive_issues(&[], 1);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::CapabilityViolation);
        assert_eq!(issues[0].source, IssueSource::Capability);
        assert!(issues[0].seq.is_none());
    }

    #[test]
    fn capability_violation_event_yields_issue_without_double_counting() {
        let events = snapshot(|r| {
            r.push(EventKind::CapabilityViolation {
                method: "secrets.get".into(),
            });
        });
        // Counter == events held: no aggregate issue, one per-event issue.
        let issues = derive_issues(&events, 1);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::CapabilityViolation);
        assert_eq!(issues[0].detail, "secrets.get");
        assert_eq!(issues[0].seq, Some(0));
    }

    #[test]
    fn restart_cap_exceeded_is_fatal() {
        let events = snapshot(|r| r.push(EventKind::RestartCapExceeded { count: 5 }));
        let issues = derive_issues(&events, 0);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::RestartCapExceeded);
        assert_eq!(issues[0].severity, Severity::Fatal);
    }

    #[test]
    fn health_timeout_yields_health_issue() {
        let events = snapshot(|r| r.push(EventKind::HealthTimeout));
        let issues = derive_issues(&events, 0);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::HealthTimeout);
        assert_eq!(issues[0].source, IssueSource::Health);
    }

    #[test]
    fn healthy_backbone_events_produce_no_issues() {
        let events = snapshot(|r| {
            r.push(EventKind::Spawned { pid: 1 });
            r.push(EventKind::StateTransition {
                to: starter_ext_spi::LifecycleState::Running,
            });
            r.push(EventKind::Stderr {
                line: "hello".into(),
            });
            r.push(EventKind::ExitedClean { code: Some(0) });
            r.push(EventKind::RestartScheduled {
                wait_ms: 100,
                total: 1,
            });
        });
        assert!(derive_issues(&events, 0).is_empty());
    }
}
