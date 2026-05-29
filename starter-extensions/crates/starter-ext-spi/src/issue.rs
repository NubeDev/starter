//! [`ExtensionIssue`] — the consolidated diagnostics read-model.
//!
//! Every known failure source for an extension (manifest-validation
//! failure, crash, restart-cap exhaustion, missed health ping, capability
//! violation, worker error) folds into one ordered list of
//! [`ExtensionIssue`]s. This is the contract type so every adapter that
//! *produces* issues — `starter-ext-host`'s [`ExtensionRecord`] for the
//! no-live-supervisor path, `starter-ext-supervisor`'s `SupervisorHandle`
//! for the live-process path — emits the same shape, and the
//! `GET /extensions/<id>/issues` handler in `starter-ext-server` can merge
//! them without knowing which crate they came from.
//!
//! Per the comprehensive-extension-management plan, **every diagnostic
//! carries a stable [`IssueCode`]** that serialises to an `ext.issue.*`
//! string the consumer maps to its own `MessageKey` catalog. There is no
//! English in the wire `code`; the human-facing context lives in the
//! free-form `detail` field, which is operator context, *not* a localised
//! key.
//!
//! [`ExtensionRecord`]: ../../starter_ext_host/struct.ExtensionRecord.html

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// How severe an [`ExtensionIssue`] is. Ordered least → most severe so a
/// consumer can `>=`-filter (`?severity=error` means "error and fatal").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational; nothing is wrong, but the operator may want to know.
    Info,
    /// A degraded condition that does not stop the extension serving.
    Warning,
    /// A failure the supervisor recovered from (e.g. a crash that
    /// restarted) or a refused capability call.
    Error,
    /// Terminal: the extension cannot serve and will not recover without
    /// an operator action (manifest invalid, restart cap exceeded).
    Fatal,
}

/// Where an [`ExtensionIssue`] was derived from. Lets the UI group issues
/// by subsystem without parsing the [`IssueCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSource {
    /// Manifest parse / semantic validation (the `Failed` record path).
    Manifest,
    /// The process supervisor's lifecycle machinery (crash, restart cap).
    Supervisor,
    /// A periodic-worker adapter (`starter-ext-workers`).
    Worker,
    /// The capability gate at the JSON-RPC wire boundary (R8).
    Capability,
    /// The supervisor's health loop (missed ping).
    Health,
}

/// Stable, non-localised diagnostic code. Serialises to an `ext.issue.*`
/// string the consumer maps onto its own message catalog (rubix uses
/// `rubix.extension.issue.*`). Adding a variant is additive within a
/// minor — UIs that don't know a new code render its `detail` generically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueCode {
    /// The `block.yaml` failed to parse or violated `deny_unknown_fields`.
    #[serde(rename = "ext.issue.manifest_invalid")]
    ManifestInvalid,
    /// A namespace-ownership rule (R4) rejected the extension.
    #[serde(rename = "ext.issue.namespace_violation")]
    NamespaceViolation,
    /// A declared capability is incompatible with what the manifest
    /// requires (R6).
    #[serde(rename = "ext.issue.capability_mismatch")]
    CapabilityMismatch,
    /// The child crashed (non-zero exit, killed after health timeout, …).
    #[serde(rename = "ext.issue.crashed")]
    Crashed,
    /// Restart intensity cap exceeded; the supervisor will not restart.
    #[serde(rename = "ext.issue.restart_cap_exceeded")]
    RestartCapExceeded,
    /// A health ping was not answered within the timeout.
    #[serde(rename = "ext.issue.health_timeout")]
    HealthTimeout,
    /// The child called a host method it did not declare a capability for.
    #[serde(rename = "ext.issue.capability_violation")]
    CapabilityViolation,
    /// A periodic worker's last run failed.
    #[serde(rename = "ext.issue.worker_failed")]
    WorkerFailed,
}

impl IssueCode {
    /// The stable wire string (`ext.issue.*`). Equivalent to serialising
    /// the variant, but as a borrowed `&'static str` for log targets and
    /// comparisons without an allocation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManifestInvalid => "ext.issue.manifest_invalid",
            Self::NamespaceViolation => "ext.issue.namespace_violation",
            Self::CapabilityMismatch => "ext.issue.capability_mismatch",
            Self::Crashed => "ext.issue.crashed",
            Self::RestartCapExceeded => "ext.issue.restart_cap_exceeded",
            Self::HealthTimeout => "ext.issue.health_timeout",
            Self::CapabilityViolation => "ext.issue.capability_violation",
            Self::WorkerFailed => "ext.issue.worker_failed",
        }
    }
}

/// One diagnostic about an extension. Produced by `ExtensionRecord` (the
/// no-supervisor path) and `SupervisorHandle` (the live path), merged and
/// sorted by `at` descending at the HTTP boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionIssue {
    /// Stable code; serialises to `ext.issue.*`. No English on the wire.
    pub code: IssueCode,
    /// Severity for filtering / colour-coding.
    pub severity: Severity,
    /// Wall-clock time the underlying event was recorded (or read time,
    /// for record-level issues that have no event timestamp).
    pub at: SystemTime,
    /// Operator-facing context — a crash reason, the refused method name,
    /// the manifest error. **Not** a localisation key; the localisation
    /// key is `code`.
    pub detail: String,
    /// Which subsystem produced the issue.
    pub source: IssueSource,
    /// Originating event-ring `seq` when the issue was derived from a ring
    /// event; `None` for record-level issues. Used by the `?since=<seq>`
    /// cursor filter.
    pub seq: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_code_serialises_to_stable_string() {
        let j = serde_json::to_string(&IssueCode::Crashed).unwrap();
        assert_eq!(j, "\"ext.issue.crashed\"");
        let back: IssueCode = serde_json::from_str("\"ext.issue.manifest_invalid\"").unwrap();
        assert_eq!(back, IssueCode::ManifestInvalid);
    }

    #[test]
    fn as_str_matches_serde() {
        for code in [
            IssueCode::ManifestInvalid,
            IssueCode::NamespaceViolation,
            IssueCode::CapabilityMismatch,
            IssueCode::Crashed,
            IssueCode::RestartCapExceeded,
            IssueCode::HealthTimeout,
            IssueCode::CapabilityViolation,
            IssueCode::WorkerFailed,
        ] {
            let serde = serde_json::to_string(&code).unwrap();
            assert_eq!(serde, format!("\"{}\"", code.as_str()));
        }
    }

    #[test]
    fn severity_orders_least_to_most_severe() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn severity_round_trips_snake_case() {
        assert_eq!(
            serde_json::to_string(&Severity::Fatal).unwrap(),
            "\"fatal\""
        );
        let s: Severity = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(s, Severity::Error);
    }

    #[test]
    fn issue_round_trips() {
        let issue = ExtensionIssue {
            code: IssueCode::CapabilityViolation,
            severity: Severity::Warning,
            at: SystemTime::UNIX_EPOCH,
            detail: "secrets.get".into(),
            source: IssueSource::Capability,
            seq: Some(7),
        };
        let j = serde_json::to_value(&issue).unwrap();
        assert_eq!(j["code"], "ext.issue.capability_violation");
        assert_eq!(j["source"], "capability");
        assert_eq!(j["seq"], 7);
        let back: ExtensionIssue = serde_json::from_value(j).unwrap();
        assert_eq!(back, issue);
    }
}
