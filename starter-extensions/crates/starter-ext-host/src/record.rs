//! [`ExtensionRecord`] — one entry in the registry.
//!
//! Records exist for both successfully-validated extensions (`state =
//! Validated`) and extensions whose manifest failed to parse or
//! validate (`state = Failed`, `failure` set). The bad-manifest smoke
//! test (SCOPE "Bad manifest is isolated to its own extension") relies
//! on this shape: the failure is *isolated to the bad record*, not
//! the entire registry.

use std::path::PathBuf;
use std::time::SystemTime;

use starter_ext_spi::{
    Error, ExtensionId, ExtensionIssue, IssueCode, IssueSource, LifecycleState, Manifest, Severity,
};

use crate::origin::BundleOrigin;

/// One extension's full state inside the registry.
#[derive(Debug, Clone)]
pub struct ExtensionRecord {
    /// Validated reverse-DNS id. Present for both successful and failed
    /// records when the manifest at least parsed far enough to surface
    /// an id; `None` when the manifest itself failed at deserialise
    /// time (in which case `id_hint` carries the directory name so an
    /// operator can still locate the bundle).
    pub id: Option<ExtensionId>,

    /// The bundle directory name. Always populated — even when the
    /// manifest failed to parse — so admin UIs can point at a path.
    pub id_hint: String,

    /// Absolute path to the bundle directory.
    pub bundle_dir: PathBuf,

    /// Lifecycle state after `Loader::commit`. `Validated` on success,
    /// `Failed` when the manifest or a semantic check rejected the
    /// extension.
    pub state: LifecycleState,

    /// Parsed manifest. `None` when parsing failed.
    pub manifest: Option<Manifest>,

    /// The error that put the record in `Failed`, if any.
    pub failure: Option<Error>,

    /// Provenance: dev source tree or installed (uploaded) bundle.
    /// Drives the uninstall handler's "may I `remove_dir_all` this?"
    /// decision. Defaults to [`BundleOrigin::Installed`] when a record
    /// is constructed without one — every existing call-site preserves
    /// its current behaviour until the loader splits scan into
    /// `scan_dev` / `scan_installs`.
    pub origin: BundleOrigin,
}

impl ExtensionRecord {
    /// `true` when the record's manifest passed every check.
    pub fn is_validated(&self) -> bool {
        matches!(self.state, LifecycleState::Validated)
    }

    /// `true` when the record is in the terminal `Failed` state.
    pub fn is_failed(&self) -> bool {
        matches!(self.state, LifecycleState::Failed)
    }

    /// Record-level diagnostics — the issues derivable from the registry
    /// alone, **without a live supervisor**.
    ///
    /// This is the no-process path: builtin, wasm, and disabled extensions
    /// never have a `SupervisorHandle`, but a record that failed manifest
    /// validation still has exactly one thing wrong with it. We surface
    /// that `failure` as a single [`Severity::Fatal`] issue so the
    /// consolidated `GET /extensions/<id>/issues` view is non-empty for a
    /// broken bundle even when nothing was ever spawned.
    ///
    /// The `code` is [`IssueCode::ManifestInvalid`] regardless of whether
    /// the underlying [`Error`] was a parse failure ([`Error::Manifest`])
    /// or a semantic rejection ([`Error::Validation`]) — both mean "this
    /// manifest cannot produce a runnable extension"; the free-form
    /// `detail` carries the specific reason. The timestamp is read-time
    /// because a record carries no event clock.
    pub fn issues(&self) -> Vec<ExtensionIssue> {
        match (self.is_failed(), &self.failure) {
            (true, Some(err)) => vec![ExtensionIssue {
                code: IssueCode::ManifestInvalid,
                severity: Severity::Fatal,
                at: SystemTime::now(),
                detail: err.to_string(),
                source: IssueSource::Manifest,
                seq: None,
            }],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed_record(failure: Error) -> ExtensionRecord {
        ExtensionRecord {
            id: None,
            id_hint: "com.acme.broken".into(),
            bundle_dir: PathBuf::from("/tmp/com.acme.broken"),
            state: LifecycleState::Failed,
            manifest: None,
            failure: Some(failure),
            origin: BundleOrigin::default(),
        }
    }

    #[test]
    fn failed_record_yields_one_fatal_manifest_invalid_issue() {
        let rec = failed_record(Error::Manifest("unknown field `frobnicate`".into()));
        let issues = rec.issues();
        assert_eq!(issues.len(), 1);
        let issue = &issues[0];
        assert_eq!(issue.code, IssueCode::ManifestInvalid);
        assert_eq!(issue.severity, Severity::Fatal);
        assert_eq!(issue.source, IssueSource::Manifest);
        assert!(issue.seq.is_none());
        assert!(issue.detail.contains("frobnicate"));
    }

    #[test]
    fn validation_failure_also_maps_to_manifest_invalid() {
        let rec = failed_record(Error::Validation("namespace not owned".into()));
        let issues = rec.issues();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, IssueCode::ManifestInvalid);
        assert_eq!(issues[0].severity, Severity::Fatal);
    }

    #[test]
    fn validated_record_has_no_issues() {
        let rec = ExtensionRecord {
            id: ExtensionId::new("com.acme.ok").ok(),
            id_hint: "com.acme.ok".into(),
            bundle_dir: PathBuf::from("/tmp/com.acme.ok"),
            state: LifecycleState::Validated,
            manifest: None,
            failure: None,
            origin: BundleOrigin::default(),
        };
        assert!(rec.issues().is_empty());
    }
}
