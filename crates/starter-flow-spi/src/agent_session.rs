//! Agent-session persistence seam (DOCS/agent/MEMORY.md).
//!
//! This module defines the [`AgentSessionStore`] trait — the single
//! seam through which `ai-agent` surfaces persist conversation turns
//! and named, versioned artifacts (page-builder trees, summaries,
//! drafts, etc.). It is intentionally separate from the basic
//! [`crate::flow::SessionStore`] key-value seam: that one stores one
//! opaque body per session; this one stores an audit log of turns
//! plus a versioned snapshot log of artifacts.
//!
//! The store records everything; the **replay strategy** (declared
//! on the `ai-agent` node config) decides what the model sees on
//! the next turn — see MEMORY.md §3 (M3, M4).
//!
//! ### Load-bearing rules
//!
//! - **M5** — turns are append-only; artifacts are versioned, never
//!   overwritten. The store assigns `seq` and `version` under a
//!   transaction. Callers never compute version numbers.
//! - **M8** — per-artifact value size is capped at
//!   [`ARTIFACT_VALUE_CAP_BYTES`]. Aggregate caps live in the
//!   replay layer, not the store.
//! - **M10/M11** — a turn and the artifacts it produced commit in
//!   one transaction via
//!   [`AgentSessionStore::append_turn_with_artifacts`]. There is no
//!   bare `append_turn` / `put_artifact` write path.
//! - **M12** — per-turn `content` size is capped at
//!   [`TURN_CONTENT_CAP_BYTES`]. Larger payloads (e.g. multi-MB
//!   tool results) must live in an artifact and be referenced by
//!   `{ "$artifact": { "key": "...", "version": N } }`.

use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Per-turn `content` size cap (M12). Serialised JSON bytes.
///
/// Larger payloads must move to an artifact and be referenced from
/// the turn by an `$artifact` envelope.
pub const TURN_CONTENT_CAP_BYTES: usize = 64 * 1024;

/// Per-artifact `value` size cap (M8). Serialised JSON bytes.
///
/// A write whose serialised value exceeds this returns
/// [`PutArtifactError::TooLarge`]. The aggregate cap for snapshot
/// replay (96 KB across `artifact_keys`) is enforced by the replay
/// layer, not the store.
pub const ARTIFACT_VALUE_CAP_BYTES: usize = 32 * 1024;

/// Current schema version for [`Turn::content`]. Bumped when the
/// turn-payload shape changes; readers handle every version they
/// know about, writers always write current (MEMORY.md §3 M2).
pub const TURN_SCHEMA_VERSION: u32 = 1;

/// Surface-owned, time-sorted identifier for an agent session
/// (MEMORY.md M7).
///
/// Backed by a UUIDv7 — the lexicographic ordering of the encoded
/// form approximates `ORDER BY created_at` (cheap pagination), and
/// the value is URL-safe.
///
/// The store treats it as an opaque key; the surface (page builder,
/// chat, debugger) generates it before the first call and threads
/// it through every subsequent request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentSessionId(pub Uuid);

impl AgentSessionId {
    /// Generate a fresh, time-sorted session id (UUIDv7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Parse a session id from its display form.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(s).map(Self)
    }
}

impl Default for AgentSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Role of a single persisted turn (MEMORY.md §3 M2 `session_turns.role`).
///
/// Mirrors the runner's view of who produced the content; the
/// engine maps richer LLM role enums down to this set at write
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TurnRole {
    /// The user / surface produced this turn (a prompt).
    User,
    /// The assistant / model produced this turn (a response).
    Assistant,
    /// A tool / system produced this turn (tool result, system
    /// note, or — when the surface opts in — a cancellation
    /// marker per M10).
    Tool,
}

/// One persisted turn read back from the store.
///
/// `seq` is monotonic per session and store-assigned (M5
/// concurrency contract). `content_bytes` is the serialised JSON
/// length; the cap enforcement happens on write (M12), the value
/// is materialised on read so callers can budget without
/// re-serialising.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Turn {
    /// Session this turn belongs to.
    pub session_id: AgentSessionId,
    /// Monotonic per-session sequence number, store-assigned.
    pub seq: u32,
    /// Producer of this turn.
    pub role: TurnRole,
    /// Normalised turn payload.
    pub content: serde_json::Value,
    /// Schema version of `content` — bump when the payload
    /// shape changes.
    pub schema_version: u32,
    /// Serialised byte length of `content`.
    pub content_bytes: u32,
    /// Reported input-token count, if the runner exposes one.
    /// CLI runners often don't report; absence is expected.
    pub tokens_in: Option<u32>,
    /// Reported output-token count, if the runner exposes one.
    pub tokens_out: Option<u32>,
    /// Wall-clock timestamp at which the store committed this row.
    pub created_at: DateTime<Utc>,
}

impl Turn {
    /// Construct a `Turn`. Used by store impls hydrating rows; the
    /// `#[non_exhaustive]` attribute prevents external struct
    /// literals, so every backend goes through this constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: AgentSessionId,
        seq: u32,
        role: TurnRole,
        content: serde_json::Value,
        schema_version: u32,
        content_bytes: u32,
        tokens_in: Option<u32>,
        tokens_out: Option<u32>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            seq,
            role,
            content,
            schema_version,
            content_bytes,
            tokens_in,
            tokens_out,
            created_at,
        }
    }
}

/// Caller-supplied turn payload for
/// [`AgentSessionStore::append_turn_with_artifacts`].
///
/// The store assigns `seq`, `content_bytes`, `created_at`, and
/// pins `schema_version` to [`TURN_SCHEMA_VERSION`]. Callers
/// supply only the producer-facing fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TurnInput {
    /// Producer of this turn.
    pub role: TurnRole,
    /// Normalised turn payload — must serialise to at most
    /// [`TURN_CONTENT_CAP_BYTES`] bytes (M12).
    pub content: serde_json::Value,
    /// Optional input-token count.
    pub tokens_in: Option<u32>,
    /// Optional output-token count.
    pub tokens_out: Option<u32>,
}

impl TurnInput {
    /// Convenience constructor.
    pub fn new(role: TurnRole, content: serde_json::Value) -> Self {
        Self {
            role,
            content,
            tokens_in: None,
            tokens_out: None,
        }
    }
}

/// Caller-supplied artifact write paired with a turn.
///
/// `parent_version` is the version this write was edited from —
/// `None` on first write of a key; `Some(prev)` for linear undo
/// or branching (MEMORY.md §3 M2). The store does not enforce
/// the parent-version relationship beyond storing it; surfaces
/// that care about lineage walk it themselves.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArtifactWrite {
    /// Artifact key (e.g. `"tree"`, `"draft"`, `"__summary"`).
    /// Keys beginning with `__` are reserved for the engine
    /// (only `__summary` is currently reserved — see M3).
    pub key: String,
    /// Artifact JSON value. Must serialise to at most
    /// [`ARTIFACT_VALUE_CAP_BYTES`] bytes (M8).
    pub value: serde_json::Value,
    /// Version this write was edited from, if known.
    pub parent_version: Option<u32>,
}

impl ArtifactWrite {
    /// Convenience constructor for the common "first write or
    /// don't-care lineage" case.
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            value,
            parent_version: None,
        }
    }

    /// Builder helper to attach a parent version.
    pub fn with_parent(mut self, parent_version: u32) -> Self {
        self.parent_version = Some(parent_version);
        self
    }
}

/// One persisted artifact row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Artifact {
    /// Session this artifact belongs to.
    pub session_id: AgentSessionId,
    /// Artifact key.
    pub key: String,
    /// Monotonic per `(session, key)` version number, store-assigned.
    pub version: u32,
    /// Version this one was edited from, if any.
    pub parent_version: Option<u32>,
    /// Artifact JSON value.
    pub value: serde_json::Value,
    /// Serialised byte length of `value`.
    pub value_bytes: u32,
    /// The turn `seq` that produced this artifact, when written
    /// via [`AgentSessionStore::append_turn_with_artifacts`].
    /// `None` for surface-initiated writes via
    /// [`AgentSessionStore::put_artifact_direct`].
    pub produced_by_seq: Option<u32>,
    /// Wall-clock timestamp at which the store committed this row.
    pub updated_at: DateTime<Utc>,
}

impl Artifact {
    /// Construct an `Artifact`. Used by store impls.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: AgentSessionId,
        key: String,
        version: u32,
        parent_version: Option<u32>,
        value: serde_json::Value,
        value_bytes: u32,
        produced_by_seq: Option<u32>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            key,
            version,
            parent_version,
            value,
            value_bytes,
            produced_by_seq,
            updated_at,
        }
    }
}
///
/// Used by [`AgentSessionStore::list_artifact_versions`] — surfaces
/// rendering an undo/version-history UI rarely need every body, so
/// the trait splits the cheap listing from the body fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArtifactMeta {
    /// Session this artifact belongs to.
    pub session_id: AgentSessionId,
    /// Artifact key.
    pub key: String,
    /// Version number.
    pub version: u32,
    /// Parent version (lineage), if any.
    pub parent_version: Option<u32>,
    /// Serialised byte length of the (unread) value.
    pub value_bytes: u32,
    /// Turn that produced this version, if any.
    pub produced_by_seq: Option<u32>,
    /// Commit timestamp.
    pub updated_at: DateTime<Utc>,
}

impl ArtifactMeta {
    /// Construct an `ArtifactMeta`. Used by store impls.
    pub fn new(
        session_id: AgentSessionId,
        key: String,
        version: u32,
        parent_version: Option<u32>,
        value_bytes: u32,
        produced_by_seq: Option<u32>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            key,
            version,
            parent_version,
            value_bytes,
            produced_by_seq,
            updated_at,
        }
    }
}
/// [`AgentSessionStore::append_turn_with_artifacts`].
///
/// Carries the store-assigned `seq` for the turn and the
/// store-assigned `version` for each artifact write in declaration
/// order, so the caller can echo them back to the client without
/// a second read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TurnReceipt {
    /// The `seq` assigned to the turn row.
    pub seq: u32,
    /// `version` assigned to each artifact in declaration order.
    /// Empty when no artifacts accompanied the turn.
    pub artifact_versions: Vec<u32>,
}

impl TurnReceipt {
    /// Construct a `TurnReceipt`. Used by store impls.
    pub fn new(seq: u32, artifact_versions: Vec<u32>) -> Self {
        Self {
            seq,
            artifact_versions,
        }
    }
}

/// One session record (the `sessions` row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AgentSession {
    /// Session id.
    pub id: AgentSessionId,
    /// Surface-defined kind (`"page-builder"`, `"chat"`, ...).
    pub kind: String,
    /// Owning principal subject, or `"system"` for unowned
    /// sessions (M9 / "Decisions made" — `owner` is required,
    /// not nullable).
    pub owner: String,
    /// Commit time of the create row.
    pub created_at: DateTime<Utc>,
    /// Last commit time of any write touching the session.
    pub updated_at: DateTime<Utc>,
    /// Free-form session metadata. Reserved keys (used by the
    /// engine when present): `provider`, `model`, `flow_id`,
    /// `client_kind`. Other keys are unrestricted; surfaces
    /// should namespace.
    pub metadata: serde_json::Value,
}

impl AgentSession {
    /// Construct an `AgentSession`. Used by store impls.
    pub fn new(
        id: AgentSessionId,
        kind: String,
        owner: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            kind,
            owner,
            created_at,
            updated_at,
            metadata,
        }
    }
}

/// Errors the [`AgentSessionStore`] write paths return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AgentSessionError {
    /// A read targeted a session that does not exist.
    #[error("agent session not found: {0}")]
    SessionNotFound(AgentSessionId),
    /// A turn `content` payload exceeded [`TURN_CONTENT_CAP_BYTES`].
    /// Move the bulky payload into an artifact and reference it
    /// from the turn with an `$artifact` envelope (M12).
    #[error("turn content exceeds cap: {bytes} > {cap}")]
    TurnTooLarge {
        /// Serialised size of the offending payload.
        bytes: usize,
        /// Configured cap ([`TURN_CONTENT_CAP_BYTES`]).
        cap: usize,
    },
    /// An artifact `value` exceeded [`ARTIFACT_VALUE_CAP_BYTES`].
    #[error("artifact {key:?} value exceeds cap: {bytes} > {cap}")]
    ArtifactTooLarge {
        /// Key of the offending artifact.
        key: String,
        /// Serialised size of the offending value.
        bytes: usize,
        /// Configured cap ([`ARTIFACT_VALUE_CAP_BYTES`]).
        cap: usize,
    },
    /// Backend / serialisation failure.
    #[error("agent session backend failure: {0}")]
    Backend(String),
}

/// Result alias for [`AgentSessionStore`].
pub type AgentSessionResult<T> = std::result::Result<T, AgentSessionError>;

/// Error variants returned by
/// [`AgentSessionStore::put_artifact_direct`].
///
/// Split from [`AgentSessionError`] because the optimistic-
/// concurrency conflict carries a typed `current` version that
/// only this write path can produce.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PutArtifactError {
    /// The caller passed `expected_prev_version: Some(_)` and the
    /// store's current latest version did not match.
    #[error("artifact version conflict; current = {current}")]
    Conflict {
        /// Latest version the store currently holds for this
        /// `(session_id, key)`.
        current: u32,
    },
    /// Value exceeded [`ARTIFACT_VALUE_CAP_BYTES`].
    #[error("artifact value exceeds cap: {bytes} > {cap}")]
    TooLarge {
        /// Serialised size of the offending value.
        bytes: usize,
        /// Configured cap.
        cap: usize,
    },
    /// Session does not exist.
    #[error("agent session not found: {0}")]
    SessionNotFound(AgentSessionId),
    /// Backend failure.
    #[error("agent session backend failure: {0}")]
    Backend(String),
}

// ---------------------------------------------------------------------
// Retention (MEMORY.md M9 / Phase M-E)
// ---------------------------------------------------------------------

/// Per-kind retention policy applied by
/// [`AgentSessionStore::sweep_retention`].
///
/// Mirrors MEMORY.md §M9 verbatim:
///
/// - [`RetentionPolicy::KeepForever`] — default; no rows ever
///   dropped by the sweeper. Use when audit / legal hold trumps
///   storage cost.
/// - [`RetentionPolicy::DeleteAfter`] — hard delete a session
///   (and, via `ON DELETE CASCADE`, every turn and artifact it
///   owns) once it has been idle for the configured duration.
///   Right for chat / debugger surfaces where the conversation
///   IS the state.
/// - [`RetentionPolicy::DeleteTurnsAfter`] — prune conversation
///   only; optionally keep just the latest artifact per key.
///   Right for the page builder where the artifact (the tree)
///   matters and the prompts that produced it can age out.
///
/// The `Duration` is **idle time** measured against
/// `agent_sessions.updated_at` (for `DeleteAfter`) or
/// `agent_session_turns.created_at` /
/// `agent_session_artifacts.updated_at` (for `DeleteTurnsAfter`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RetentionPolicy {
    /// Never delete. Default — every session lives until the
    /// surface explicitly drops it via
    /// [`AgentSessionStore::delete`].
    #[default]
    KeepForever,
    /// Hard delete sessions idle for at least `ttl`.
    DeleteAfter {
        /// Maximum idle duration (against `sessions.updated_at`).
        ttl: chrono::Duration,
    },
    /// Drop turns idle for at least `ttl`; optionally also drop
    /// every artifact row except the latest version per key for
    /// sessions whose turns have aged out.
    DeleteTurnsAfter {
        /// Maximum idle duration for the turn rows.
        ttl: chrono::Duration,
        /// When `true`, after pruning the turns also collapse the
        /// artifact history of each touched session down to the
        /// latest version per key. When `false`, artifacts are
        /// left intact (the safe default for "forget the prompts,
        /// keep every save").
        keep_latest_artifact: bool,
    },
}


/// Report from [`AgentSessionStore::sweep_retention`].
///
/// Counters are summed across the sweep; zero is a valid result
/// (nothing to prune yet). Hosts surface these in logs / metrics
/// to detect runaway growth.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RetentionSweepReport {
    /// Number of `agent_sessions` rows hard-deleted.
    pub sessions_deleted: u64,
    /// Number of `agent_session_turns` rows deleted (either via
    /// the session cascade or the turn-only sweep).
    pub turns_deleted: u64,
    /// Number of `agent_session_artifacts` rows deleted (cascade
    /// or `keep_latest_artifact` collapse).
    pub artifacts_deleted: u64,
}

impl RetentionSweepReport {
    /// Sum two reports, useful when iterating over multiple
    /// `(kind, policy)` pairs in a host scheduler.
    pub fn merge(self, other: Self) -> Self {
        Self {
            sessions_deleted: self.sessions_deleted + other.sessions_deleted,
            turns_deleted: self.turns_deleted + other.turns_deleted,
            artifacts_deleted: self.artifacts_deleted + other.artifacts_deleted,
        }
    }
}

/// Persistence seam for agent sessions (DOCS/agent/MEMORY.md M1).
///
/// One trait, two backends ([`starter-store-sqlite`] +
/// `starter-store-postgres`), picked at composition time via
/// `Engine::builder().with_session_store(...)`. The trait shape is
/// load-bearing: it makes the M5 concurrency contract, the M8/M12
/// size caps, and the M10/M11 transactionality unrepresentable to
/// violate from a backend impl.
///
/// All read methods return `Ok(None)` / empty for missing rows;
/// only the write paths return [`AgentSessionError::SessionNotFound`]
/// when the targeted session is absent.
#[async_trait]
pub trait AgentSessionStore: Send + Sync + 'static {
    // ----- lifecycle -----

    /// Create a new session row. `owner` is required and must be
    /// `"system"` for unowned sessions (MEMORY.md "Decisions
    /// made"). Caller-supplied `id` so the surface can hand the
    /// id back to the client before the first turn lands.
    async fn create(
        &self,
        id: AgentSessionId,
        kind: &str,
        owner: &str,
        metadata: serde_json::Value,
    ) -> AgentSessionResult<()>;

    /// Fetch a session record by id, or `None` if absent.
    async fn get(&self, id: AgentSessionId) -> AgentSessionResult<Option<AgentSession>>;

    /// Delete a session and every turn / artifact that belongs to
    /// it (cascade). Used by the retention pruner and by surfaces
    /// that explicitly drop a session.
    async fn delete(&self, id: AgentSessionId) -> AgentSessionResult<()>;

    // ----- writes (M10, M11) -----

    /// Single transactional write of a turn plus any artifacts it
    /// produced. Store assigns `seq` and every artifact `version`.
    /// This is the **only** write path used by the agent loop.
    ///
    /// Returns [`AgentSessionError::TurnTooLarge`] /
    /// [`AgentSessionError::ArtifactTooLarge`] without writing if
    /// any cap is exceeded — partial commits are not possible.
    async fn append_turn_with_artifacts(
        &self,
        id: AgentSessionId,
        turn: TurnInput,
        artifacts: &[ArtifactWrite],
    ) -> AgentSessionResult<TurnReceipt>;

    /// Surface-initiated artifact write (e.g. manual rename,
    /// save-as) outside of an agent turn. Optimistic concurrency
    /// via `expected_prev_version`:
    ///
    /// - `None` — unconditional; the store appends as the next
    ///   version.
    /// - `Some(prev)` — append only if the current latest version
    ///   equals `prev`; otherwise returns
    ///   [`PutArtifactError::Conflict`] with the actual current
    ///   version.
    ///
    /// Use the receipt-style return for the assigned version.
    async fn put_artifact_direct(
        &self,
        id: AgentSessionId,
        key: &str,
        value: serde_json::Value,
        expected_prev_version: Option<u32>,
    ) -> Result<u32, PutArtifactError>;

    // ----- reads -----

    /// List turns for a session, ordered by `seq` ascending.
    /// `since_seq` is exclusive — pass the highest `seq` already
    /// observed to page forward. `limit` caps the page size; the
    /// trait does not impose a default cap (backends may, for
    /// safety).
    async fn list_turns(
        &self,
        id: AgentSessionId,
        since_seq: Option<u32>,
        limit: Option<u32>,
    ) -> AgentSessionResult<Vec<Turn>>;

    /// Latest version of a single named artifact, or `None` if
    /// the key has never been written for this session.
    ///
    /// This is the read path the "snapshot" replay strategy
    /// (M4 / §5 step 4) and the
    /// `GET /api/sessions/:id/artifacts/:key` endpoint use.
    async fn latest_artifact(
        &self,
        id: AgentSessionId,
        key: &str,
    ) -> AgentSessionResult<Option<Artifact>>;

    /// Fetch a specific `(key, version)` body. Used by undo /
    /// history surfaces.
    async fn artifact_at(
        &self,
        id: AgentSessionId,
        key: &str,
        version: u32,
    ) -> AgentSessionResult<Option<Artifact>>;

    /// Enumerate every version of one key, newest first, without
    /// fetching the bodies. Used by undo / version-picker UIs.
    async fn list_artifact_versions(
        &self,
        id: AgentSessionId,
        key: &str,
    ) -> AgentSessionResult<Vec<ArtifactMeta>>;

    // ----- retention (M9 / Phase M-E) -----

    /// Apply a [`RetentionPolicy`] to every session of `kind` and
    /// report how many rows were affected.
    ///
    /// Drives MEMORY.md §M9: hosts call this from their own
    /// scheduled task (or from a periodic engine maintenance
    /// loop, when one exists) — the store does not own the
    /// clock. `now` is supplied so tests can pin a deterministic
    /// cutoff; production callers pass [`chrono::Utc::now`].
    ///
    /// The default implementation is a no-op so backends that
    /// haven't ported retention yet (e.g. early `starter-store-
    /// postgres` builds) still compile against this trait.
    /// SQLite overrides it with the real cascading delete.
    async fn sweep_retention(
        &self,
        kind: &str,
        policy: &RetentionPolicy,
        now: DateTime<Utc>,
    ) -> AgentSessionResult<RetentionSweepReport> {
        let _ = (kind, policy, now);
        Ok(RetentionSweepReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_roundtrips_through_string() {
        let id = AgentSessionId::new();
        let s = id.to_string();
        let parsed = AgentSessionId::parse(&s).expect("uuid parses");
        assert_eq!(id, parsed);
    }

    #[test]
    fn session_ids_sort_lexicographically_by_creation_time() {
        // UUIDv7 carries a millisecond timestamp in its high bits;
        // two ids generated >1ms apart must compare in creation
        // order under string ordering. The 2 ms spacing avoids
        // the same-millisecond bucket where ordering falls back
        // to the random tail.
        let a = AgentSessionId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = AgentSessionId::new();
        assert!(a.to_string() < b.to_string(), "{a} should sort before {b}");
    }

    #[test]
    fn turn_input_constructor_defaults_tokens_to_none() {
        let t = TurnInput::new(TurnRole::User, serde_json::json!({"text": "hi"}));
        assert!(t.tokens_in.is_none());
        assert!(t.tokens_out.is_none());
    }

    #[test]
    fn artifact_write_with_parent_attaches_lineage() {
        let w = ArtifactWrite::new("tree", serde_json::json!({})).with_parent(3);
        assert_eq!(w.parent_version, Some(3));
    }

    #[test]
    fn caps_are_documented_and_non_zero() {
        // Const-time invariant; the test exists to lock the
        // relationship in CI so a future tweak to one cap can't
        // silently invert the other.
        const _: () = assert!(TURN_CONTENT_CAP_BYTES > 0);
        const _: () = assert!(ARTIFACT_VALUE_CAP_BYTES > 0);
        // Per MEMORY.md M8 + M12: turn cap is the larger of the
        // two (turns carry tool-call traces; artifacts hold a
        // single named state).
        const _: () = assert!(TURN_CONTENT_CAP_BYTES >= ARTIFACT_VALUE_CAP_BYTES);
    }
}
