//! Definition-layer contracts: events, sources, and policy.
//!
//! Per `DOCS/flow/scope/hot-reload.md`: every flow definition edit
//! funnels through one chokepoint
//! (`DefinitionManager::publish` in `starter-flow`); the types here
//! are the wire shapes that chokepoint produces (events on the
//! definition bus, audit row source tags) and the policy enum that
//! controls how a structural edit applies to in-flight runs
//! (`ApplyPolicy`, read from the flow body per HR4 / R3).
//!
//! No runtime logic, no I/O — these are the contracts the engine
//! crate and any future adapter (REST handlers, CLI, file-watch,
//! extensions) agree on.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::flow::{FlowId, FlowRevisionId};

/// How a draft published through `DefinitionManager::publish` reached
/// the chokepoint.
///
/// Per HR7 (file-watch is one publisher among many) and the
/// Observability section: this tag is stamped on every emitted
/// [`FlowDefinitionEvent`] and persisted in the `FlowStore` revisions
/// table (audit column) so a single SQL query can answer
/// *"which surface published this revision?"*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DefinitionSource {
    /// REST handler, programmatic API call from a host binary, or any
    /// other in-process publisher that doesn't fit a more specific
    /// variant.
    Api,
    /// CLI command (`starter-cli flow publish ...`).
    Cli,
    /// Host-dir file watcher (`starter-flow-watch`, lands Phase HR-5).
    /// `path` is the on-disk path the watcher event referenced.
    File {
        /// Absolute path of the file that triggered the publish.
        path: PathBuf,
    },
    /// Extension-contributed publisher (Phase 6 `starter-ext-flow`).
    /// `id` is the extension's reverse-DNS contribution id.
    Extension {
        /// Reverse-DNS id of the contributing extension.
        id: String,
    },
}

impl DefinitionSource {
    /// Short stable tag suitable for the
    /// `flow.definition.publish` tracing span's `source` field and
    /// the `FlowStore` revisions audit column.
    ///
    /// Format mirrors the Observability section in `hot-reload.md`:
    /// `"api" | "cli" | "file:<path>" | "extension:<id>"`.
    pub fn audit_tag(&self) -> String {
        match self {
            Self::Api => "api".to_owned(),
            Self::Cli => "cli".to_owned(),
            Self::File { path } => format!("file:{}", path.display()),
            Self::Extension { id } => format!("extension:{id}"),
        }
    }
}

/// Per-flow policy controlling how a structural edit applies to
/// in-flight runs of the previous revision.
///
/// Per HR4: read from the *previous* revision's body, not owned by
/// the engine (R3 — *"the engine is a reader of policies, never an
/// owner"*). The engine has no `match` arm over the policy names; it
/// looks up the value once per swap and dispatches via the
/// strategies registered against the enum.
///
/// Defaults to [`Self::Drain`] (HR4 D-HR3) which matches the
/// Codeless-shape — short-lived staged runs don't benefit from
/// mid-run topology changes. Rubix-shape flows opt into the other
/// variants explicitly via a config slot on the flow body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum ApplyPolicy {
    /// In-flight runs finish on the snapshot they hold; new runs use
    /// the new topology. Default.
    #[default]
    Drain,
    /// Cancel in-flight runs (R13 `RunCancel`); safe-state walks
    /// writable outputs (R12); new runs (re-triggered by the caller
    /// or by `trigger.event`) use the new topology.
    Restart,
    /// Migrate in-flight runs where ownership of the affected slot
    /// didn't shift; fall back to [`Self::Restart`] for everything
    /// else. The classifier is conservative — see HR4 for the full
    /// safety bar.
    LiveMigrate,
}

/// What edit-kind the diff classifier produced for a publish.
///
/// Carried on [`FlowDefinitionEvent::RevisionPublished`] and stamped
/// on the `flow.definition.publish` tracing span's `kind` field per
/// the Observability section. The full classifier (with the actual
/// slot-write deltas for [`Self::SettingsOnly`] and [`Self::Mixed`])
/// is internal to `starter-flow` — Phase HR-2 land — so the SPI
/// surface is the kind enum only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum EditKindTag {
    /// First publish for a flow — no previous revision to diff
    /// against; the swap is unconditionally an atomic mount per HR4
    /// first-publish corner case.
    Initial,
    /// Only config-slot values changed. Settings path: per-delta
    /// `write_slot` with `WriteSlotOpts::config()`.
    Settings,
    /// Links, nodes, or kind ids changed. Structural path: build a
    /// new topology and atomically swap.
    Structural,
    /// Both settings and structural deltas. Structural swap first,
    /// then per-delta settings writes (HR3 order).
    Mixed,
}

/// Events emitted on the engine's definition bus.
///
/// Per the Observability section, every transition is a tracing span
/// **and** an event on the broadcast bus the engine exposes via
/// `Engine::definition_events()`. UI canvas surfaces, REST handlers,
/// and tests subscribe to drive their own reactivity.
///
/// `#[non_exhaustive]` so new event variants (e.g. a future
/// `DraftValidated` for HR-extra preview flows) are additive.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FlowDefinitionEvent {
    /// A new immutable revision was written through HR1's chokepoint.
    /// `prev_head` is `None` for a flow's first publish.
    RevisionPublished {
        /// The flow this revision belongs to.
        flow: FlowId,
        /// The newly-written revision id.
        revision: FlowRevisionId,
        /// The previous head, if any.
        prev_head: Option<FlowRevisionId>,
        /// How this publish reached the chokepoint.
        source: DefinitionSource,
        /// Classifier output (Initial / Settings / Structural / Mixed).
        kind: EditKindTag,
    },
    /// An idempotent publish hit HR1's short-circuit (the canonical
    /// body hash matched the current head). No new revision, no swap,
    /// no audit row. Surfaced so callers can distinguish "save was a
    /// no-op" from "save failed".
    PublishShortCircuited {
        /// The flow.
        flow: FlowId,
        /// The head revision the publish collapsed onto.
        head: FlowRevisionId,
        /// How this publish reached the chokepoint.
        source: DefinitionSource,
    },
    /// A draft was rejected — validation, resolve, or settings
    /// failure. `FlowStore` is untouched; `ActiveTopology` is
    /// untouched (HR6).
    Rejected {
        /// The flow id the draft targeted.
        flow: FlowId,
        /// How this publish reached the chokepoint.
        source: DefinitionSource,
        /// Human-readable summary of why; the structured error
        /// surfaces on the call return as the typed error from
        /// `DefinitionManager::publish`.
        reason: String,
    },
    /// A previously-`ResolveFailed` flow successfully resolved against
    /// the live `NodeKindRegistry` and was mounted into
    /// `ActiveTopology`. Fired by HR8 on
    /// `NodeKindRegistry::register` and by HR5 on boot.
    Mounted {
        /// The flow.
        flow: FlowId,
        /// The revision that resolved successfully.
        revision: FlowRevisionId,
    },
    /// `TopologyResolver::resolve` errored — e.g. a kind referenced
    /// by the head revision is not registered, or a stored revision
    /// no longer validates against the current kind's schema.
    /// The flow stays unmounted; consumers can re-publish or wait
    /// for the kind to register (HR8).
    ResolveFailed {
        /// The flow.
        flow: FlowId,
        /// The revision that failed to resolve.
        revision: FlowRevisionId,
        /// Human-readable description of the failure.
        error: String,
    },
    /// A flow was removed (e.g. file deleted under a host-dir
    /// watcher). The `ActiveTopology` entry is gone; in-flight runs
    /// drain per `apply_policy`.
    Removed {
        /// The flow that was removed.
        flow: FlowId,
        /// How the removal reached the chokepoint.
        source: DefinitionSource,
    },
    /// A structural swap landed: the per-flow active pointer moved
    /// from `from_revision` to `to_revision`. Fired by HR2 / HR4 on
    /// every successful structural or mixed publish.
    SwapApplied {
        /// The flow.
        flow: FlowId,
        /// Previous active revision (`None` on first mount).
        from_revision: Option<FlowRevisionId>,
        /// New active revision.
        to_revision: FlowRevisionId,
        /// Policy read from the *previous* revision that governed the
        /// swap (HR4 — policy-edit corner case).
        apply_policy: ApplyPolicy,
    },
    /// A node kind was deregistered and the engine revoked every
    /// active topology that referenced it (HR8).
    KindRevoked {
        /// The flow whose active topology was revoked.
        flow: FlowId,
        /// The kind id that was deregistered.
        kind: String,
        /// Policy that governed the revocation.
        apply_policy: ApplyPolicy,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_tag_shape_matches_hot_reload_doc() {
        assert_eq!(DefinitionSource::Api.audit_tag(), "api");
        assert_eq!(DefinitionSource::Cli.audit_tag(), "cli");
        assert_eq!(
            DefinitionSource::File {
                path: PathBuf::from("/flows/a.yaml")
            }
            .audit_tag(),
            "file:/flows/a.yaml"
        );
        assert_eq!(
            DefinitionSource::Extension {
                id: "com.acme.weather".to_owned()
            }
            .audit_tag(),
            "extension:com.acme.weather"
        );
    }

    #[test]
    fn apply_policy_default_is_drain() {
        assert_eq!(ApplyPolicy::default(), ApplyPolicy::Drain);
    }

    #[test]
    fn apply_policy_kebab_case_serde() {
        let s = serde_json::to_string(&ApplyPolicy::LiveMigrate).unwrap();
        assert_eq!(s, "\"live-migrate\"");
        let back: ApplyPolicy = serde_json::from_str("\"drain\"").unwrap();
        assert_eq!(back, ApplyPolicy::Drain);
    }
}
