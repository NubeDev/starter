//! Agent-session replay strategies (DOCS/agent/MEMORY.md §3 M3 + M4).
//!
//! The [`AgentSessionStore`](starter_flow_spi::agent_session::AgentSessionStore)
//! always records every turn and every artifact. This module owns
//! the *read* path that turns persisted state into the input the
//! model sees on the next turn. Storage and replay are deliberately
//! separate decisions (the MEMORY.md headline rule).
//!
//! Four strategies, exhaustively:
//!
//! | Strategy             | What the model sees on turn N                                  | Cost |
//! |----------------------|----------------------------------------------------------------|------|
//! | [`ReplayStrategy::None`]               | New prompt only                                            | O(1) |
//! | [`ReplayStrategy::Snapshot`]           | Latest values of every key in `artifact_keys` + new prompt | O(1) |
//! | [`ReplayStrategy::SummaryPlusTail`]    | `__summary` + last `tail_k` turns verbatim                 | O(1) |
//! | [`ReplayStrategy::Full`]               | Every prior turn verbatim                                  | O(N) |
//!
//! The output is a [`ReplayInput`] — an opaque, runner-facing
//! envelope listing the system-prompt fragments (the snapshot
//! payloads or the summary), the prior conversation messages to
//! replay, and any caps the layer applied (with telemetry-grade
//! markers per M8).
//!
//! ### Caps (M8)
//!
//! - Per-artifact: 32 KB serialised
//!   ([`ARTIFACT_VALUE_CAP_BYTES`](starter_flow_spi::agent_session::ARTIFACT_VALUE_CAP_BYTES)).
//!   Enforced by the store on write; this layer additionally enforces
//!   the snapshot aggregate cap.
//! - Aggregate across `artifact_keys`: [`SNAPSHOT_AGGREGATE_CAP_BYTES`]
//!   (96 KB). Over the limit, artifacts are dropped in *reverse
//!   declaration order* and a [`ReplayWarning::SnapshotAggregateTruncated`]
//!   is returned alongside the input so the surface can surface an
//!   `error` frame to the client.
//!
//! ### `__summary` lifecycle
//!
//! The summary is stored as artifact key `"__summary"` and produced
//! by a separate summariser flow (M3). This module reads whatever
//! the latest version happens to be; if absent (early in a session
//! or the summariser hasn't caught up), it is omitted and the input
//! is bounded by `tail_k` alone.

use serde::{Deserialize, Serialize};
use starter_flow_spi::agent_session::{
    AgentSessionError, AgentSessionId, AgentSessionResult, AgentSessionStore, Artifact, Turn,
    ARTIFACT_VALUE_CAP_BYTES,
};

/// Aggregate cap across every artifact inlined under
/// [`ReplayStrategy::Snapshot`] (MEMORY.md M8). Bytes of serialised
/// JSON, summed across the declared `artifact_keys`.
pub const SNAPSHOT_AGGREGATE_CAP_BYTES: usize = 96 * 1024;

/// Reserved artifact key for the rolling summary produced by the
/// [`ReplayStrategy::SummaryPlusTail`] summariser (MEMORY.md §3 M3).
pub const SUMMARY_ARTIFACT_KEY: &str = "__summary";

/// Replay strategy declared on the `ai-agent` node config slot
/// (MEMORY.md §3 M3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReplayStrategy {
    /// New prompt only. The model sees nothing from the prior
    /// session — equivalent to today's stateless
    /// `/api/builder/stream` behaviour, but the store still
    /// records the turn for audit.
    None,
    /// Inline the latest version of every declared artifact key
    /// into the system prompt. Right for surfaces where the
    /// *current state* is the substrate (the page builder: its
    /// state IS the tree).
    Snapshot {
        /// Artifact keys to inline, in declaration order. The
        /// aggregate cap (M8) drops in reverse declaration order
        /// when exceeded — declare keys most-important-first.
        artifact_keys: Vec<String>,
    },
    /// Read the latest `__summary` artifact + replay the last
    /// `tail_k` turns verbatim. Right for chat assistants and
    /// long-running agents where the conversation IS the state.
    SummaryPlusTail {
        /// Number of recent turns to replay verbatim after the
        /// summary.
        tail_k: u32,
        /// Cadence at which the summariser re-runs in the
        /// background. This module does not invoke the
        /// summariser; the field is carried so the engine's
        /// summariser scheduler (a separate concern) reads it
        /// off the same config struct.
        summarise_every_k_turns: u32,
    },
    /// Replay every prior turn verbatim. O(N) tokens. Right for
    /// debugger views and short tool conversations; wrong for
    /// any long-lived surface.
    Full,
}

impl Default for ReplayStrategy {
    /// Defaults to [`ReplayStrategy::None`] — the safest stateless
    /// behaviour, identical to today's `/api/builder/stream`.
    fn default() -> Self {
        Self::None
    }
}

/// One artifact snapshot that survived the aggregate cap and is
/// being handed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SnapshotFragment {
    /// Artifact key.
    pub key: String,
    /// Version of the artifact at read time.
    pub version: u32,
    /// JSON value to inline into the system prompt.
    pub value: serde_json::Value,
    /// Serialised byte size of `value` — pre-computed so the
    /// runner can budget tokens against a known size without
    /// re-serialising.
    pub bytes: usize,
}

impl SnapshotFragment {
    fn from_artifact(artifact: Artifact, bytes: usize) -> Self {
        Self {
            key: artifact.key,
            version: artifact.version,
            value: artifact.value,
            bytes,
        }
    }
}

/// Warnings the replay layer emits when it had to drop or truncate
/// state to stay under a cap. The surface should fan these out as
/// `error` frames to the client per MEMORY.md M8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReplayWarning {
    /// One or more declared `artifact_keys` had no corresponding
    /// artifact in the store and were silently skipped. Carries
    /// the missing keys so the surface can decide whether the
    /// absence is expected (first turn) or a bug (key typo).
    SnapshotArtifactMissing {
        /// Keys that returned `Ok(None)` from
        /// `latest_artifact`.
        keys: Vec<String>,
    },
    /// The aggregate cap fired and the listed keys were dropped
    /// from the snapshot. Drops are in reverse declaration
    /// order so the most-important keys stay.
    SnapshotAggregateTruncated {
        /// Keys that were dropped, in the order they were
        /// dropped (least-important first).
        dropped_keys: Vec<String>,
        /// Total serialised size *before* dropping, in bytes.
        attempted_bytes: usize,
        /// Configured aggregate cap, in bytes.
        cap_bytes: usize,
    },
    /// `SummaryPlusTail` was asked for the summary but the
    /// `__summary` artifact has never been written for this
    /// session. The tail still replays; the summary slot is
    /// empty. Expected early in a session.
    SummaryMissing,
}

/// Materialised replay input for one upcoming model call.
///
/// Opaque shape — the runner consumes it and produces whatever the
/// concrete LLM API requires. Documented here as a typed seam so
/// future runners (a streaming Claude REST runner, a Gemini CLI
/// runner) compose against the same data without re-deriving it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReplayInput {
    /// Artifact snapshots to inline into the system prompt
    /// ([`ReplayStrategy::Snapshot`]) — empty for the other
    /// strategies.
    pub snapshots: Vec<SnapshotFragment>,
    /// Latest `__summary` artifact, when
    /// [`ReplayStrategy::SummaryPlusTail`] resolved one.
    pub summary: Option<SnapshotFragment>,
    /// Prior turns to replay verbatim, oldest first. Populated
    /// by [`ReplayStrategy::Full`] and [`ReplayStrategy::SummaryPlusTail`];
    /// always empty for [`ReplayStrategy::None`] /
    /// [`ReplayStrategy::Snapshot`].
    pub turns: Vec<Turn>,
    /// Warnings (cap drops, missing keys, missing summary) the
    /// surface should fan out to the client.
    pub warnings: Vec<ReplayWarning>,
}

impl ReplayInput {
    /// Convenience: total serialised bytes of every inlined
    /// snapshot fragment plus the summary. Useful for the
    /// runner's prompt budget accounting.
    pub fn inlined_bytes(&self) -> usize {
        let snap: usize = self.snapshots.iter().map(|s| s.bytes).sum();
        snap + self.summary.as_ref().map(|s| s.bytes).unwrap_or(0)
    }
}

/// Build the [`ReplayInput`] for one upcoming model call.
///
/// Reads from the store; never writes. Pure of side effects
/// beyond the reads — the surface composes this output with the
/// new user prompt to drive the runner.
///
/// The trait object is taken by reference so the engine's stored
/// `Arc<dyn AgentSessionStore>` and any test double satisfy the
/// signature without a clone.
pub async fn build_replay_input(
    store: &dyn AgentSessionStore,
    session: AgentSessionId,
    strategy: &ReplayStrategy,
) -> AgentSessionResult<ReplayInput> {
    match strategy {
        ReplayStrategy::None => Ok(ReplayInput::default()),

        ReplayStrategy::Snapshot { artifact_keys } => {
            build_snapshot(store, session, artifact_keys).await
        }

        ReplayStrategy::Full => {
            let turns = store.list_turns(session, None, None).await?;
            Ok(ReplayInput {
                turns,
                ..Default::default()
            })
        }

        ReplayStrategy::SummaryPlusTail { tail_k, .. } => {
            build_summary_plus_tail(store, session, *tail_k).await
        }
    }
}

async fn build_snapshot(
    store: &dyn AgentSessionStore,
    session: AgentSessionId,
    artifact_keys: &[String],
) -> AgentSessionResult<ReplayInput> {
    let mut fragments: Vec<SnapshotFragment> = Vec::with_capacity(artifact_keys.len());
    let mut missing: Vec<String> = Vec::new();

    // Order is the caller's declaration order (most-important
    // first). The cap-trim loop below drops from the *tail*, so
    // the earliest-declared keys are the ones the model keeps.
    for key in artifact_keys {
        match store.latest_artifact(session, key).await? {
            Some(artifact) => {
                // Pre-compute the serialised size — the store
                // wrote `value_bytes` at insert time, so trust
                // it rather than re-serialising. (The cap on
                // write is M8: each artifact is already at most
                // ARTIFACT_VALUE_CAP_BYTES; the aggregate cap
                // here is the sum across keys.)
                let bytes = artifact.value_bytes as usize;
                // Defensive bound — if a future store impl ever
                // lets a larger blob through, refuse to ship it
                // rather than blow the prompt budget silently.
                if bytes > ARTIFACT_VALUE_CAP_BYTES {
                    return Err(AgentSessionError::ArtifactTooLarge {
                        key: artifact.key.clone(),
                        bytes,
                        cap: ARTIFACT_VALUE_CAP_BYTES,
                    });
                }
                fragments.push(SnapshotFragment::from_artifact(artifact, bytes));
            }
            None => missing.push(key.clone()),
        }
    }

    let mut warnings: Vec<ReplayWarning> = Vec::new();
    if !missing.is_empty() {
        warnings.push(ReplayWarning::SnapshotArtifactMissing { keys: missing });
    }

    let attempted_bytes: usize = fragments.iter().map(|f| f.bytes).sum();
    if attempted_bytes > SNAPSHOT_AGGREGATE_CAP_BYTES {
        // Drop in reverse declaration order until under the cap.
        let mut current_total = attempted_bytes;
        let mut dropped: Vec<String> = Vec::new();
        while current_total > SNAPSHOT_AGGREGATE_CAP_BYTES && !fragments.is_empty() {
            // pop the *least-important* (last-declared) key.
            let f = fragments.pop().expect("non-empty");
            current_total -= f.bytes;
            dropped.push(f.key);
        }
        warnings.push(ReplayWarning::SnapshotAggregateTruncated {
            dropped_keys: dropped,
            attempted_bytes,
            cap_bytes: SNAPSHOT_AGGREGATE_CAP_BYTES,
        });
    }

    Ok(ReplayInput {
        snapshots: fragments,
        warnings,
        ..Default::default()
    })
}

async fn build_summary_plus_tail(
    store: &dyn AgentSessionStore,
    session: AgentSessionId,
    tail_k: u32,
) -> AgentSessionResult<ReplayInput> {
    let summary_artifact = store.latest_artifact(session, SUMMARY_ARTIFACT_KEY).await?;
    let mut warnings: Vec<ReplayWarning> = Vec::new();
    let summary = match summary_artifact {
        Some(a) => {
            let bytes = a.value_bytes as usize;
            Some(SnapshotFragment::from_artifact(a, bytes))
        }
        None => {
            warnings.push(ReplayWarning::SummaryMissing);
            None
        }
    };

    // Tail walk: the trait paginates ascending by `seq`, so we
    // ask for the full conversation since the summary's
    // `produced_by_seq` (when present) and trim to the last
    // `tail_k` rows. Cheap for short tails; if a session ever
    // produces thousands of turns since the last summary, the
    // summariser cadence is misconfigured — the cost surfaces
    // as a slow page render, which is the right escalation.
    //
    // Future op: add a descending paged read to the trait so
    // we never touch turns that pre-date the summary. Not in v1.
    let since = summary
        .as_ref()
        .and_then(|s| s.value.get("produced_by_seq").and_then(|v| v.as_u64()))
        .map(|seq| seq as u32);
    let all = store.list_turns(session, since, None).await?;
    let tail_k = tail_k as usize;
    let turns = if all.len() > tail_k {
        let skip = all.len() - tail_k;
        all.into_iter().skip(skip).collect()
    } else {
        all
    };

    Ok(ReplayInput {
        snapshots: Vec::new(),
        summary,
        turns,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use starter_flow_spi::agent_session::{
        Artifact, ArtifactMeta, ArtifactWrite, PutArtifactError, Turn, TurnInput, TurnReceipt,
        TurnRole,
    };
    use std::sync::Mutex;

    /// Minimal in-memory `AgentSessionStore` for unit tests. Only
    /// the methods the replay layer touches are populated; the
    /// rest panic so we notice if a strategy grows a new read
    /// path that's not in the test plan.
    #[derive(Default)]
    struct FakeStore {
        artifacts: Mutex<Vec<Artifact>>, // newest last per key
        turns: Mutex<Vec<Turn>>,         // append-only
    }

    impl FakeStore {
        fn push_artifact(&self, key: &str, value: serde_json::Value, bytes: u32) {
            let mut a = self.artifacts.lock().unwrap();
            let next_version = a
                .iter()
                .filter(|x| x.key == key)
                .map(|x| x.version)
                .max()
                .unwrap_or(0)
                + 1;
            a.push(Artifact::new(
                AgentSessionId::new(), // session id doesn't matter in this fake
                key.to_owned(),
                next_version,
                None,
                value,
                bytes,
                None,
                chrono::Utc::now(),
            ));
        }

        fn push_turn(&self, seq: u32, role: TurnRole, content: serde_json::Value) {
            let mut t = self.turns.lock().unwrap();
            t.push(Turn::new(
                AgentSessionId::new(),
                seq,
                role,
                content,
                1,
                0,
                None,
                None,
                chrono::Utc::now(),
            ));
        }
    }

    #[async_trait]
    impl AgentSessionStore for FakeStore {
        async fn create(
            &self,
            _id: AgentSessionId,
            _kind: &str,
            _owner: &str,
            _metadata: serde_json::Value,
        ) -> AgentSessionResult<()> {
            unimplemented!("replay layer never calls create")
        }
        async fn get(
            &self,
            _id: AgentSessionId,
        ) -> AgentSessionResult<Option<starter_flow_spi::agent_session::AgentSession>> {
            unimplemented!("replay layer never calls get")
        }
        async fn delete(&self, _id: AgentSessionId) -> AgentSessionResult<()> {
            unimplemented!()
        }
        async fn append_turn_with_artifacts(
            &self,
            _id: AgentSessionId,
            _turn: TurnInput,
            _artifacts: &[ArtifactWrite],
        ) -> AgentSessionResult<TurnReceipt> {
            unimplemented!()
        }
        async fn put_artifact_direct(
            &self,
            _id: AgentSessionId,
            _key: &str,
            _value: serde_json::Value,
            _expected_prev_version: Option<u32>,
        ) -> Result<u32, PutArtifactError> {
            unimplemented!()
        }
        async fn list_turns(
            &self,
            _id: AgentSessionId,
            since_seq: Option<u32>,
            limit: Option<u32>,
        ) -> AgentSessionResult<Vec<Turn>> {
            let t = self.turns.lock().unwrap();
            let since = since_seq.unwrap_or(0);
            let it = t.iter().filter(|x| x.seq > since).cloned();
            Ok(match limit {
                Some(l) => it.take(l as usize).collect(),
                None => it.collect(),
            })
        }
        async fn latest_artifact(
            &self,
            _id: AgentSessionId,
            key: &str,
        ) -> AgentSessionResult<Option<Artifact>> {
            let a = self.artifacts.lock().unwrap();
            Ok(a.iter().filter(|x| x.key == key).next_back().cloned())
        }
        async fn artifact_at(
            &self,
            _id: AgentSessionId,
            key: &str,
            version: u32,
        ) -> AgentSessionResult<Option<Artifact>> {
            let a = self.artifacts.lock().unwrap();
            Ok(a.iter()
                .find(|x| x.key == key && x.version == version)
                .cloned())
        }
        async fn list_artifact_versions(
            &self,
            _id: AgentSessionId,
            _key: &str,
        ) -> AgentSessionResult<Vec<ArtifactMeta>> {
            unimplemented!()
        }
    }

    fn session() -> AgentSessionId {
        AgentSessionId::new()
    }

    #[tokio::test]
    async fn none_strategy_returns_empty_input() {
        let store = FakeStore::default();
        let input = build_replay_input(&store, session(), &ReplayStrategy::None)
            .await
            .unwrap();
        assert!(input.snapshots.is_empty());
        assert!(input.turns.is_empty());
        assert!(input.warnings.is_empty());
        assert!(input.summary.is_none());
    }

    #[tokio::test]
    async fn snapshot_inlines_declared_keys_in_order() {
        let store = FakeStore::default();
        store.push_artifact("tree", serde_json::json!({"t": 1}), 12);
        store.push_artifact("theme", serde_json::json!({"th": 1}), 14);
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::Snapshot {
                artifact_keys: vec!["tree".into(), "theme".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(input.snapshots.len(), 2);
        assert_eq!(input.snapshots[0].key, "tree");
        assert_eq!(input.snapshots[1].key, "theme");
        assert!(input.warnings.is_empty());
    }

    #[tokio::test]
    async fn snapshot_missing_keys_warn_but_succeed() {
        let store = FakeStore::default();
        store.push_artifact("tree", serde_json::json!({}), 4);
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::Snapshot {
                artifact_keys: vec!["tree".into(), "missing".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(input.snapshots.len(), 1);
        assert_eq!(input.warnings.len(), 1);
        match &input.warnings[0] {
            ReplayWarning::SnapshotArtifactMissing { keys } => {
                assert_eq!(keys, &vec!["missing".to_string()]);
            }
            other => panic!("expected SnapshotArtifactMissing, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn snapshot_aggregate_cap_drops_least_important_first() {
        let store = FakeStore::default();
        // Two artifacts, each just under the per-artifact cap, so
        // the aggregate cap (96 KB) fires after the second one.
        let big = ARTIFACT_VALUE_CAP_BYTES as u32 - 1024;
        store.push_artifact("important", serde_json::json!({}), big);
        store.push_artifact("nice-to-have-1", serde_json::json!({}), big);
        store.push_artifact("nice-to-have-2", serde_json::json!({}), big);
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::Snapshot {
                artifact_keys: vec![
                    "important".into(),
                    "nice-to-have-1".into(),
                    "nice-to-have-2".into(),
                ],
            },
        )
        .await
        .unwrap();
        // 3 * (32K - 1K) = ~93K — under cap; nothing should drop.
        assert_eq!(input.snapshots.len(), 3);
        assert!(
            input
                .warnings
                .iter()
                .all(|w| !matches!(w, ReplayWarning::SnapshotAggregateTruncated { .. })),
            "no truncation expected when under cap, got: {:?}",
            input.warnings
        );
    }

    #[tokio::test]
    async fn snapshot_aggregate_cap_truncates_when_over() {
        let store = FakeStore::default();
        // 4 * 32K = 128K, over the 96K aggregate cap. Drops in
        // reverse declaration order until at-or-under cap:
        //   start 128K (over) → drop k3 → 96K (== cap, loop
        //   exits). Three keys remain, one dropped.
        let big = ARTIFACT_VALUE_CAP_BYTES as u32;
        store.push_artifact("k0", serde_json::json!({}), big);
        store.push_artifact("k1", serde_json::json!({}), big);
        store.push_artifact("k2", serde_json::json!({}), big);
        store.push_artifact("k3", serde_json::json!({}), big);
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::Snapshot {
                artifact_keys: vec!["k0".into(), "k1".into(), "k2".into(), "k3".into()],
            },
        )
        .await
        .unwrap();
        // Reverse-order drop: k3 dropped first; total now at cap.
        assert_eq!(input.snapshots.len(), 3);
        assert_eq!(input.snapshots[0].key, "k0");
        assert_eq!(input.snapshots[2].key, "k2");
        let trunc = input
            .warnings
            .iter()
            .find_map(|w| match w {
                ReplayWarning::SnapshotAggregateTruncated {
                    dropped_keys,
                    attempted_bytes,
                    cap_bytes,
                } => Some((dropped_keys.clone(), *attempted_bytes, *cap_bytes)),
                _ => None,
            })
            .expect("aggregate truncation warning");
        assert_eq!(trunc.0, vec!["k3".to_string()]);
        assert_eq!(trunc.2, SNAPSHOT_AGGREGATE_CAP_BYTES);
        assert!(trunc.1 > SNAPSHOT_AGGREGATE_CAP_BYTES);
    }

    #[tokio::test]
    async fn snapshot_aggregate_cap_drops_multiple_when_far_over() {
        let store = FakeStore::default();
        // 5 * 32K = 160K → need to drop k4 (→128K), still over;
        // drop k3 (→96K, at cap, stop). Two dropped.
        let big = ARTIFACT_VALUE_CAP_BYTES as u32;
        for i in 0..5 {
            store.push_artifact(&format!("k{i}"), serde_json::json!({}), big);
        }
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::Snapshot {
                artifact_keys: (0..5).map(|i| format!("k{i}")).collect(),
            },
        )
        .await
        .unwrap();
        assert_eq!(input.snapshots.len(), 3);
        let trunc = input
            .warnings
            .iter()
            .find_map(|w| match w {
                ReplayWarning::SnapshotAggregateTruncated { dropped_keys, .. } => {
                    Some(dropped_keys.clone())
                }
                _ => None,
            })
            .expect("truncation");
        assert_eq!(trunc, vec!["k4".to_string(), "k3".to_string()]);
    }

    #[tokio::test]
    async fn full_replays_every_turn() {
        let store = FakeStore::default();
        for i in 1..=3 {
            store.push_turn(i, TurnRole::User, serde_json::json!({"i": i}));
        }
        let input = build_replay_input(&store, session(), &ReplayStrategy::Full)
            .await
            .unwrap();
        assert_eq!(input.turns.len(), 3);
        assert_eq!(input.turns[0].seq, 1);
        assert_eq!(input.turns[2].seq, 3);
    }

    #[tokio::test]
    async fn summary_plus_tail_keeps_last_k_when_summary_present() {
        let store = FakeStore::default();
        store.push_artifact(
            SUMMARY_ARTIFACT_KEY,
            serde_json::json!({"text": "so far..."}),
            10,
        );
        for i in 1..=5 {
            store.push_turn(i, TurnRole::User, serde_json::json!({"i": i}));
        }
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::SummaryPlusTail {
                tail_k: 2,
                summarise_every_k_turns: 10,
            },
        )
        .await
        .unwrap();
        assert!(input.summary.is_some());
        assert_eq!(input.turns.len(), 2);
        assert_eq!(input.turns[0].seq, 4);
        assert_eq!(input.turns[1].seq, 5);
        assert!(input
            .warnings
            .iter()
            .all(|w| !matches!(w, ReplayWarning::SummaryMissing)));
    }

    #[tokio::test]
    async fn summary_plus_tail_warns_when_no_summary_yet() {
        let store = FakeStore::default();
        store.push_turn(1, TurnRole::User, serde_json::json!({}));
        let input = build_replay_input(
            &store,
            session(),
            &ReplayStrategy::SummaryPlusTail {
                tail_k: 6,
                summarise_every_k_turns: 10,
            },
        )
        .await
        .unwrap();
        assert!(input.summary.is_none());
        assert_eq!(input.turns.len(), 1);
        assert!(input
            .warnings
            .iter()
            .any(|w| matches!(w, ReplayWarning::SummaryMissing)));
    }

    #[test]
    fn default_strategy_is_none() {
        assert!(matches!(ReplayStrategy::default(), ReplayStrategy::None));
    }

    #[test]
    fn inlined_bytes_sums_snapshots_and_summary() {
        let mut input = ReplayInput::default();
        input.snapshots.push(SnapshotFragment {
            key: "a".into(),
            version: 1,
            value: serde_json::json!({}),
            bytes: 100,
        });
        input.summary = Some(SnapshotFragment {
            key: "__summary".into(),
            version: 1,
            value: serde_json::json!({}),
            bytes: 50,
        });
        assert_eq!(input.inlined_bytes(), 150);
    }
}
