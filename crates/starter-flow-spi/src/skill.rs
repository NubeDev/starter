//! Skill-selection seam for the `ai-agent` node kind.
//!
//! Per `DOCS/flow/scope/SCOPE.md` "Skills bind to the `ai-agent` node
//! kind": skill selection runs once per outer flow run and threads
//! through every `ai-agent` node in the flow as a frozen
//! [`SkillSelection`]. D-F4.4 locks the trait shape and ships a
//! default [`NullSkillSelector`] returning [`SkillSelection::None`];
//! the real content-hash-backed selector lives in a future
//! `starter-skills` crate that is **not** a workspace member yet.
//!
//! Phase 4 invariant: the [`SkillSelection`] handed to each node's
//! [`crate::node::NodeCtx`] is the one the engine resolved at run
//! start; re-running the selector mid-run is forbidden (the
//! `skill_quarantine_survives_bundle_update_through_a_flow` smoke
//! proves this end-to-end).

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::node::{IdError, KindId, SlotMap};
use crate::Principal;

/// Reverse-DNS skill identifier, validated on construction (R10).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SkillId(String);

impl SkillId {
    /// Parse a string as a skill id; returns [`IdError`] on invalid
    /// reverse-DNS shape.
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        // Re-use the KindId validator — same reverse-DNS rule.
        let _ = KindId::new(s.clone())?;
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for SkillId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SkillId> for String {
    fn from(value: SkillId) -> Self {
        value.0
    }
}

/// Mounted skill resource pointer the `ai-agent` body exposes to the
/// LLM as a readable file. Only the fields the engine needs to thread
/// the selection through; the full schema lives in the future
/// `starter-skills` crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResourceRef {
    /// Opaque URI the host resolves to a file/blob.
    pub uri: String,
    /// blake3 content hash of the resource bytes at selection time.
    /// Quarantine invariant: a mid-flow bundle update changes this
    /// hash, but the in-flight run keeps the value it selected with.
    pub content_hash: String,
}

impl ResourceRef {
    /// Construct a [`ResourceRef`]. Required because the struct is
    /// `#[non_exhaustive]` for forward-compatibility, which forbids
    /// struct-literal construction from outside this crate (the
    /// `starter-skills` registry consumes this constructor).
    pub fn new(uri: impl Into<String>, content_hash: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            content_hash: content_hash.into(),
        }
    }
}

/// One skill selection threaded through every `ai-agent` node in a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SkillSelection {
    /// No skill was selected for this run. Default for the
    /// [`NullSkillSelector`].
    None,
    /// Skill was selected; every `ai-agent` node in the run intersects
    /// its allowed-tools against [`Selected::allowed_tools`] per
    /// D-F4.5.
    Selected {
        /// The selected skill id.
        skill_id: SkillId,
        /// Tool-id allowlist contributed by this skill.
        allowed_tools: Vec<KindId>,
        /// Resources mounted into each `ai-agent` invocation context.
        resources: Vec<ResourceRef>,
        /// blake3 content hash of the skill bundle at selection time
        /// (quarantine invariant; see [`ResourceRef::content_hash`]).
        content_hash: String,
    },
}

impl SkillSelection {
    /// Static `&SkillSelection::None` reference, suitable as the
    /// default `skill` argument to [`crate::node::NodeCtx::new`] when
    /// no selector is configured on the engine.
    pub const NONE: &'static SkillSelection = &SkillSelection::None;
}

/// Selector the engine runs once per outer flow run.
///
/// Default impl: [`NullSkillSelector`] returns [`SkillSelection::None`].
/// The real content-hash-backed selector lives in the future
/// `starter-skills` crate (D-F4.4).
#[async_trait]
pub trait SkillSelector: Send + Sync + 'static {
    /// Resolve a selection for the given run input + principal.
    /// Called exactly once per `FlowRunner::start`; the result is
    /// frozen for the duration of the run.
    async fn select(
        &self,
        input: &SlotMap,
        principal: &Principal,
    ) -> Result<SkillSelection, SkillError>;
}

/// No-op [`SkillSelector`] that always returns [`SkillSelection::None`].
///
/// Engine default when no selector is registered via
/// `Engine::with_skill_selector(...)`. Phase 4 ships only this impl;
/// the real selector lives in the future `starter-skills` crate.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSkillSelector;

#[async_trait]
impl SkillSelector for NullSkillSelector {
    async fn select(
        &self,
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        Ok(SkillSelection::None)
    }
}

/// Typed error returned by [`SkillSelector::select`].
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[error("skill selection failed [{code}]: {message}")]
#[non_exhaustive]
pub struct SkillError {
    /// Short machine-readable code (e.g. `"bundle_not_found"`).
    pub code: String,
    /// Human-readable message; safe to surface in logs.
    pub message: String,
}

impl SkillError {
    /// Construct a new [`SkillError`].
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
