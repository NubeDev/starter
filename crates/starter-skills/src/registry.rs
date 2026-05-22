//! [`SkillRegistry`] + [`SkillRegistryBuilder`].
//!
//! The registry is the load-and-quarantine state machine. It walks
//! every configured source (a `load_dir(...)` root, a
//! `load_dir_quarantined(...)` root, an `extend(...)` batch of
//! contributed bundles), hashes each bundle, consults the
//! [`ApprovalStore`], and partitions the result into two maps:
//! `approved` and `quarantined`.
//!
//! ## Trust matrix (R-skills-3, normative)
//!
//! | Source                                  | Frontmatter trust | Effective trust            |
//! |-----------------------------------------|-------------------|----------------------------|
//! | [`SkillRegistryBuilder::load_dir`]      | `approved` / absent | **approved**             |
//! | [`SkillRegistryBuilder::load_dir`]      | `quarantined`     | **quarantined**            |
//! | [`SkillRegistryBuilder::load_dir_quarantined`] | any        | **quarantined** (always)   |
//! | [`SkillRegistryBuilder::extend`]        | any               | **quarantined** (always)   |
//!
//! An [`ApprovalStore`] row keyed on `(skill_id, bundle_hash)`
//! promotes a quarantined entry to approved. Hash mismatch
//! re-quarantines on the next [`SkillRegistry::reload`].
//!
//! ## What this module does **not** do
//!
//! - It does not implement [`starter_flow_spi::SkillSelector`].
//!   That lands in Phase 4 alongside the three selector impls.
//! - It does not verify resource hashes at mount time. That lands
//!   in Phase 4b inside `crates/starter-flow-nodes/src/ai_agent.rs`.
//! - It does not watch the filesystem. Reload is operator-driven
//!   (R-skills-8: "refresh via explicit `reload()` only").

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use starter_flow_spi::node::KindId;
use starter_flow_spi::skill::{ResourceRef, SkillId};
use starter_flow_spi::Principal;
use starter_spi::ai::AiRunner;
use thiserror::Error;

use crate::approval::hash_bundle;
use crate::bundle::{load_bundle, Bundle};
use crate::error::SkillParseError;
use crate::parser::Trust;
use crate::selector::{KeywordSkillSelector, LlmSkillSelector, SelectorStrategy};
use crate::store::{ApprovalRow, ApprovalStore, ApprovalStoreError, InMemoryApprovalStore};

/// A loaded skill the registry serves to selectors.
///
/// `Skill` is intentionally cheap to clone: every field is either a
/// small owned value, an [`Arc<str>`], or a small `Vec`. The body
/// (the bytes the model will eventually see) is held by `Arc<str>`
/// so a `select()` call can hand back a [`SkillSelection`] without
/// copying the body string.
///
/// [`SkillSelection`]: starter_flow_spi::skill::SkillSelection
#[derive(Debug, Clone)]
pub struct Skill {
    /// Validated reverse-DNS id.
    pub id: SkillId,
    /// Free-form description (surfaced to selectors and operator UIs).
    pub description: String,
    /// Tool-id allowlist contributed by this skill (intersected with
    /// the host's `ToolRegistry` at run time per agent R3).
    pub allowed_tools: Vec<KindId>,
    /// Documentary model preference (S-D3: best-effort).
    pub model_hint: Option<String>,
    /// Effective trust after the matrix has been applied. Always
    /// matches the map the skill is stored in.
    pub trust: Trust,
    /// Markdown body verbatim. `Arc` so handing it to a `SkillSelection`
    /// is a refcount bump, not a copy. **No templating, ever**.
    pub body: Arc<str>,
    /// Resources mounted by the `ai-agent` body. Each entry carries a
    /// per-file `content_hash` the mount-time check (Phase 4b) compares
    /// against the bytes on disk.
    pub resources: Vec<ResourceRef>,
    /// blake3 hash of the whole bundle (R-skills-2). The approval
    /// store keys on this value; mid-run quarantine relies on it.
    pub bundle_hash: String,
}

/// One bundle a host extension wants the registry to include.
///
/// Extension-contributed skills are **always quarantined** regardless
/// of the frontmatter `trust:` field (R-skills-3 row 3). The host's
/// operator must approve them explicitly through
/// [`SkillRegistry::approve`] before they become selectable.
///
/// In v1 a contribution is just a bundle directory on disk. Phase 6
/// (the `starter-ext-flow` wiring) constructs these from the
/// `contributes.skills` field of an extension manifest; the path
/// points at the bundle the extension shipped or extracted.
#[derive(Debug, Clone)]
pub struct ContributedSkill {
    /// Bundle root directory. Must contain a `SKILL.md`.
    pub bundle_root: PathBuf,
}

impl ContributedSkill {
    /// Convenience constructor.
    pub fn new(bundle_root: impl Into<PathBuf>) -> Self {
        Self {
            bundle_root: bundle_root.into(),
        }
    }
}

/// Errors [`SkillRegistryBuilder::build`] and
/// [`SkillRegistry::reload`] can return. Every variant surfaces enough
/// context (path or skill id) for the operator to find and fix the
/// offending bundle without grepping logs.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    /// A configured load directory does not exist or is not a
    /// readable directory.
    #[error("load directory {dir} is not a readable directory: {source}")]
    InvalidLoadDir {
        /// The directory the loader was asked to walk.
        dir: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A bundle under a load directory failed to parse.
    #[error(transparent)]
    Bundle(#[from] SkillParseError),

    /// `hash_bundle` failed (I/O during the walk).
    #[error("hash bundle {bundle_root}: {source}")]
    Hash {
        /// Bundle directory whose hash could not be computed.
        bundle_root: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The approval store backend returned an error during
    /// build/reload/approve/revoke. The trait says approve/revoke
    /// can fail; the registry surfaces the failure through this arm
    /// rather than swallowing it.
    #[error(transparent)]
    Store(#[from] ApprovalStoreError),

    /// Two bundles claimed the same `SkillId`. The registry keeps
    /// the *first* (in source-traversal order) and rejects the
    /// duplicate so a typo in an extension cannot silently shadow
    /// a first-party skill.
    #[error("duplicate skill id `{skill_id}` (first at {first}, dup at {dup})")]
    DuplicateSkillId {
        /// The conflicting skill id.
        skill_id: SkillId,
        /// Path of the first bundle to register the id.
        first: PathBuf,
        /// Path of the duplicate bundle.
        dup: PathBuf,
    },
}

/// What the registry was configured to load. Reload re-walks every
/// source from scratch, which is exactly the point: nothing about a
/// previous load can mask a current change.
#[derive(Debug, Clone)]
enum Source {
    /// `load_dir(root)` — bundle-per-subdirectory walk, frontmatter
    /// trust honoured (approved by default unless `trust:
    /// quarantined`).
    LoadDir(PathBuf),
    /// `load_dir_quarantined(root)` — staging directory, every
    /// bundle is forced to quarantined regardless of frontmatter.
    LoadDirQuarantined(PathBuf),
    /// `extend(batch)` — extension-contributed bundles, every one
    /// is forced to quarantined.
    Extend(Vec<ContributedSkill>),
}

/// Internal map state kept under a single `RwLock`. `BTreeMap` for
/// deterministic iteration order (helps tests and operator UIs).
#[derive(Debug, Default)]
struct State {
    approved: BTreeMap<SkillId, Arc<Skill>>,
    quarantined: BTreeMap<SkillId, Arc<Skill>>,
    /// Bundle directory each skill id was loaded from, indexed for
    /// `reload()` and for `LoadError::DuplicateSkillId` diagnostics.
    bundle_paths: HashMap<SkillId, PathBuf>,
}

/// Builder for [`SkillRegistry`]. See module docs for the trust
/// matrix each loader method implements.
pub struct SkillRegistryBuilder {
    approval_store: Option<Arc<dyn ApprovalStore>>,
    sources: Vec<Source>,
    /// Explicit strategy override. When `None`, the builder picks
    /// `LlmSkillSelector` (if `ai_runner` is set) or
    /// `KeywordSkillSelector` (otherwise).
    strategy: Option<Arc<dyn SelectorStrategy>>,
    /// Optional `AiRunner` used to construct the default
    /// `LlmSkillSelector` when no explicit strategy is configured.
    ai_runner: Option<Arc<dyn AiRunner>>,
}

impl SkillRegistryBuilder {
    /// Construct an empty builder. The caller must at minimum call
    /// [`Self::with_approval_store`] before [`Self::build`].
    pub fn new() -> Self {
        Self {
            approval_store: None,
            sources: Vec::new(),
            strategy: None,
            ai_runner: None,
        }
    }

    /// Wire the [`ApprovalStore`]. Required: building without one
    /// would mean every quarantined bundle stays quarantined forever
    /// with no way to promote.
    pub fn with_approval_store<S>(mut self, store: S) -> Self
    where
        S: ApprovalStore,
    {
        self.approval_store = Some(Arc::new(store));
        self
    }

    /// Wire the [`ApprovalStore`] as an already-shared `Arc`. Useful
    /// when the host holds a single store across multiple registries
    /// (e.g. one per scope) but wants the rows to live in one place.
    pub fn with_approval_store_arc(mut self, store: Arc<dyn ApprovalStore>) -> Self {
        self.approval_store = Some(store);
        self
    }

    /// Explicit selector strategy. Overrides the
    /// `ai_runner` + default-resolution path described on
    /// [`crate::selector`]. Hosts that want a custom strategy (for
    /// example, the "record candidates" wrapper used by the
    /// quarantine smoke) wire one in this way.
    pub fn with_default_selector<S>(mut self, strategy: S) -> Self
    where
        S: SelectorStrategy,
    {
        self.strategy = Some(Arc::new(strategy));
        self
    }

    /// Variant of [`Self::with_default_selector`] that accepts an
    /// already-shared `Arc<dyn SelectorStrategy>`.
    pub fn with_default_selector_arc(mut self, strategy: Arc<dyn SelectorStrategy>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Wire an [`AiRunner`] used to build the default
    /// [`LlmSkillSelector`] when no explicit strategy is configured.
    /// Ignored when [`Self::with_default_selector`] is also called.
    pub fn with_ai_runner(mut self, runner: Arc<dyn AiRunner>) -> Self {
        self.ai_runner = Some(runner);
        self
    }

    /// Queue a `load_dir` source. Honours the frontmatter trust
    /// field (R-skills-3 rows 1 + 2).
    pub fn load_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.sources.push(Source::LoadDir(dir.into()));
        self
    }

    /// Queue a `load_dir_quarantined` source. Every bundle under
    /// `dir` is forced to quarantined regardless of frontmatter
    /// (R-skills-3: staging directory semantics).
    pub fn load_dir_quarantined(mut self, dir: impl Into<PathBuf>) -> Self {
        self.sources.push(Source::LoadDirQuarantined(dir.into()));
        self
    }

    /// Queue an extension-contributed batch. Always quarantined
    /// (R-skills-3 row 3) — extensions cannot ship pre-approved
    /// skills.
    pub fn extend(mut self, batch: Vec<ContributedSkill>) -> Self {
        self.sources.push(Source::Extend(batch));
        self
    }

    /// Build the registry. Walks every queued source, hashes every
    /// bundle, consults the [`ApprovalStore`], partitions into
    /// approved + quarantined.
    pub async fn build(self) -> Result<SkillRegistry, LoadError> {
        let approval_store = self
            .approval_store
            .unwrap_or_else(|| Arc::new(InMemoryApprovalStore::new()));

        // Default-strategy resolution per `crate::selector` module
        // docs: explicit override > AiRunner-backed Llm > Keyword.
        let strategy: Arc<dyn SelectorStrategy> = match (self.strategy, self.ai_runner) {
            (Some(s), _) => s,
            (None, Some(runner)) => Arc::new(LlmSkillSelector::new(runner)),
            (None, None) => Arc::new(KeywordSkillSelector::new()),
        };

        let registry = SkillRegistry {
            inner: Arc::new(RwLock::new(State::default())),
            approval_store,
            sources: Arc::new(self.sources),
            strategy,
        };
        registry.reload().await?;
        Ok(registry)
    }
}

impl Default for SkillRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SkillRegistryBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillRegistryBuilder")
            .field("has_approval_store", &self.approval_store.is_some())
            .field("source_count", &self.sources.len())
            .finish()
    }
}

/// Skill registry — entry point for the Phase 4 selector and the
/// Phase 4b on-mount hash check.
#[derive(Clone)]
pub struct SkillRegistry {
    inner: Arc<RwLock<State>>,
    approval_store: Arc<dyn ApprovalStore>,
    sources: Arc<Vec<Source>>,
    /// Selector strategy the [`starter_flow_spi::skill::SkillSelector`]
    /// impl dispatches to after filtering quarantined bundles out
    /// (R-skills-3). Frozen at `build()` time; not swappable at
    /// runtime by design — the engine pins one selector per run, so
    /// hot-swapping a strategy mid-run would break the
    /// once-per-run-selection contract.
    strategy: Arc<dyn SelectorStrategy>,
}

impl std::fmt::Debug for SkillRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().expect("registry poisoned");
        f.debug_struct("SkillRegistry")
            .field("approved", &inner.approved.len())
            .field("quarantined", &inner.quarantined.len())
            .field("sources", &self.sources.len())
            .finish()
    }
}

impl SkillRegistry {
    /// Start a builder.
    pub fn builder() -> SkillRegistryBuilder {
        SkillRegistryBuilder::new()
    }

    /// Borrow the underlying [`ApprovalStore`]. Hosts use this to
    /// share the same store across registries.
    pub fn approval_store(&self) -> Arc<dyn ApprovalStore> {
        Arc::clone(&self.approval_store)
    }

    /// Borrow the configured [`SelectorStrategy`]. Exposed so the
    /// `SkillSelector` impl in [`crate::selector`] can dispatch
    /// without re-resolving the default strategy.
    pub fn strategy(&self) -> Arc<dyn SelectorStrategy> {
        Arc::clone(&self.strategy)
    }

    /// Every approved skill, in deterministic (`BTreeMap`) order.
    pub fn list(&self) -> Vec<Arc<Skill>> {
        self.inner
            .read()
            .expect("registry poisoned")
            .approved
            .values()
            .cloned()
            .collect()
    }

    /// Every quarantined skill, in deterministic order. Operators
    /// drive the approval UI off this list.
    pub fn list_quarantined(&self) -> Vec<Arc<Skill>> {
        self.inner
            .read()
            .expect("registry poisoned")
            .quarantined
            .values()
            .cloned()
            .collect()
    }

    /// Look up a skill by id. Searches the approved map first, then
    /// the quarantined map. Selectors **must not** call this for
    /// dispatch — they want approved-only — but operator UIs and
    /// the on-mount check need to see quarantined entries.
    pub fn get(&self, id: &SkillId) -> Option<Arc<Skill>> {
        let inner = self.inner.read().expect("registry poisoned");
        inner
            .approved
            .get(id)
            .cloned()
            .or_else(|| inner.quarantined.get(id).cloned())
    }

    /// Record an approval row and promote the matching quarantined
    /// skill to approved if its current hash equals `bundle_hash`.
    ///
    /// Recording an approval for a hash that does **not** match any
    /// loaded bundle is allowed (the operator may be pre-approving a
    /// not-yet-reloaded version) — the row lands in the store, but
    /// no in-memory promotion happens.
    pub async fn approve(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
        principal: &Principal,
    ) -> Result<(), LoadError> {
        let row = ApprovalRow::now(
            skill_id.clone(),
            bundle_hash.to_owned(),
            principal.subject.clone(),
        );
        self.approval_store.record(row).await?;
        self.repartition_one(skill_id, bundle_hash, true);
        Ok(())
    }

    /// Revoke an approval row and demote the matching skill to
    /// quarantined if its current hash equals `bundle_hash`.
    pub async fn revoke(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
        _principal: &Principal,
    ) -> Result<(), LoadError> {
        self.approval_store.revoke(skill_id, bundle_hash).await?;
        self.repartition_one(skill_id, bundle_hash, false);
        Ok(())
    }

    /// Re-walk every configured source, rehash every bundle, and
    /// re-partition against the current approval store snapshot.
    ///
    /// Drift never mutates the approval store (R-skills-7): if a
    /// previously-approved hash is no longer present on disk, the
    /// prior row stays in the store (inert) and the bundle's new
    /// hash simply has no row, so the matrix re-quarantines it.
    pub async fn reload(&self) -> Result<(), LoadError> {
        let mut next = State::default();
        for source in self.sources.iter() {
            match source {
                Source::LoadDir(root) => {
                    self.walk_load_dir(root, false, &mut next).await?;
                }
                Source::LoadDirQuarantined(root) => {
                    self.walk_load_dir(root, true, &mut next).await?;
                }
                Source::Extend(batch) => {
                    for contrib in batch {
                        self.load_one(&contrib.bundle_root, true, &mut next).await?;
                    }
                }
            }
        }
        *self.inner.write().expect("registry poisoned") = next;
        Ok(())
    }

    // -------- internals --------

    async fn walk_load_dir(
        &self,
        root: &Path,
        force_quarantine: bool,
        next: &mut State,
    ) -> Result<(), LoadError> {
        let read = fs::read_dir(root).map_err(|source| LoadError::InvalidLoadDir {
            dir: root.to_path_buf(),
            source,
        })?;
        let mut bundle_dirs: Vec<PathBuf> = Vec::new();
        for entry in read {
            let entry = entry.map_err(|source| LoadError::InvalidLoadDir {
                dir: root.to_path_buf(),
                source,
            })?;
            let file_type = entry
                .file_type()
                .map_err(|source| LoadError::InvalidLoadDir {
                    dir: root.to_path_buf(),
                    source,
                })?;
            if !file_type.is_dir() {
                continue;
            }
            let candidate = entry.path();
            if candidate.join("SKILL.md").is_file() {
                bundle_dirs.push(candidate);
            }
            // Subdirectories without a SKILL.md are ignored — the
            // walker is one level deep on purpose, otherwise a
            // misplaced SKILL.md in `node_modules/` would surprise
            // operators.
        }
        // Sort so the load order (and therefore the
        // DuplicateSkillId diagnostics) is deterministic.
        bundle_dirs.sort();
        for dir in bundle_dirs {
            self.load_one(&dir, force_quarantine, next).await?;
        }
        Ok(())
    }

    async fn load_one(
        &self,
        bundle_root: &Path,
        force_quarantine: bool,
        next: &mut State,
    ) -> Result<(), LoadError> {
        let bundle = load_bundle(bundle_root)?;
        let bundle_hash = hash_bundle(bundle_root).map_err(|source| LoadError::Hash {
            bundle_root: bundle_root.to_path_buf(),
            source,
        })?;

        let id = bundle.skill.id.clone();
        if let Some(first) = next.bundle_paths.get(&id) {
            return Err(LoadError::DuplicateSkillId {
                skill_id: id,
                first: first.clone(),
                dup: bundle_root.to_path_buf(),
            });
        }

        let resources = resource_refs(&bundle);
        let frontmatter_trust = bundle.skill.trust;

        let approved_row = self
            .approval_store
            .lookup(&id, &bundle_hash)
            .await
            .map_err(LoadError::Store)?;

        let effective_trust = if force_quarantine {
            // R-skills-3 row 3 + load_dir_quarantined: a forced
            // quarantine source still respects an approval row,
            // because the operator may have explicitly approved this
            // exact bundle hash after vetting the extension.
            if approved_row.is_some() {
                Trust::Approved
            } else {
                Trust::Quarantined
            }
        } else {
            match frontmatter_trust {
                Trust::Approved => Trust::Approved,
                Trust::Quarantined => {
                    if approved_row.is_some() {
                        Trust::Approved
                    } else {
                        Trust::Quarantined
                    }
                }
            }
        };

        let skill = Arc::new(Skill {
            id: id.clone(),
            description: bundle.skill.description.clone(),
            allowed_tools: bundle.skill.allowed_tools.clone(),
            model_hint: bundle.skill.model_hint.clone(),
            trust: effective_trust,
            body: Arc::<str>::from(bundle.skill.body.as_str()),
            resources,
            bundle_hash,
        });

        match effective_trust {
            Trust::Approved => {
                next.approved.insert(id.clone(), skill);
            }
            Trust::Quarantined => {
                next.quarantined.insert(id.clone(), skill);
            }
        }
        next.bundle_paths.insert(id, bundle_root.to_path_buf());
        Ok(())
    }

    /// Move a single skill between the approved/quarantined maps if
    /// its currently-loaded hash matches `bundle_hash`. Used by
    /// `approve` (promote=true) and `revoke` (promote=false).
    fn repartition_one(&self, skill_id: &SkillId, bundle_hash: &str, promote: bool) {
        let mut inner = self.inner.write().expect("registry poisoned");
        let from = if promote {
            &mut inner.quarantined
        } else {
            &mut inner.approved
        };
        // Borrow split: take the entry out, mutate trust, put it back
        // into the other map. We only move when the in-memory hash
        // matches the row's hash — operators can pre-approve a hash
        // that doesn't match the current bytes, and that must not
        // promote the wrong bundle.
        let Some(arc) = from.get(skill_id).cloned() else {
            return;
        };
        if arc.bundle_hash != bundle_hash {
            return;
        }
        from.remove(skill_id);
        let mut owned: Skill = (*arc).clone();
        owned.trust = if promote {
            Trust::Approved
        } else {
            Trust::Quarantined
        };
        let to = if promote {
            &mut inner.approved
        } else {
            &mut inner.quarantined
        };
        to.insert(skill_id.clone(), Arc::new(owned));
    }
}

/// Build the `Vec<ResourceRef>` for a loaded bundle. The per-file
/// hash uses the same line-ending normalisation as
/// [`hash_bundle`][crate::approval::hash_bundle] so the Phase 4b
/// on-mount check (which reads the bytes off disk and rehashes) can
/// compare against this frozen value byte-for-byte.
///
/// The [`ResourceRef::uri`] is rewritten from the frontmatter's
/// bundle-relative form (e.g. `file://greeting.txt`) into a
/// fully-qualified `file:///abs/.../bundle/greeting.txt` URI so the
/// `ai-agent` body — which only sees the
/// [`starter_flow_spi::skill::SkillSelection`], never the registry —
/// can resolve the URI to disk bytes at mount time without
/// re-consulting the registry for the bundle root.
fn resource_refs(bundle: &Bundle) -> Vec<ResourceRef> {
    // Canonicalise once per bundle so the URI is a stable absolute
    // path on whichever filesystem ai-agent eventually mounts from.
    // `canonicalize` requires the path to exist; we just hashed the
    // bundle, so it does. Fall back to the original `bundle.root`
    // (which may already be absolute) if canonicalisation fails for
    // a reason we cannot recover from — the on-mount hash check will
    // surface the drift either way.
    let bundle_root_abs = fs::canonicalize(&bundle.root).unwrap_or_else(|_| bundle.root.clone());
    let mut out = Vec::with_capacity(bundle.resources.len());
    for r in &bundle.resources {
        let bytes = if crate::approval::is_text_path(&r.relative_path) {
            crate::approval::normalise_line_endings_pub(&r.bytes)
        } else {
            r.bytes.to_vec()
        };
        let content_hash = blake3::hash(&bytes).to_hex().to_string();
        let abs = bundle_root_abs.join(&r.relative_path);
        // `display()` is sufficient here because the load-time walker
        // already rejected non-UTF-8 path components (see
        // `relative_forward_slash` in approval.rs).
        let uri = format!("file://{}", abs.display());
        out.push(ResourceRef::new(uri, content_hash));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    // ----- minimal tempdir helper (avoid `tempfile` dep) -----

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let base = std::env::temp_dir().join(format!(
                "starter-skills-reg-{tag}-{}-{nanos}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).unwrap();
            Self(base)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    fn principal() -> Principal {
        Principal {
            subject: "operator-alice".into(),
            role: starter_spi::auth::Role::Admin,
            scopes: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    fn approved_skill_md(id: &str) -> String {
        format!("---\nid: {id}\ndescription: smoke\ntrust: approved\n---\nbody\n")
    }

    fn quarantined_skill_md(id: &str) -> String {
        format!("---\nid: {id}\ndescription: smoke\ntrust: quarantined\n---\nbody\n")
    }

    /// Smoke 1 from the stage brief: "Extension-contributed skill is
    /// quarantined regardless of frontmatter."
    ///
    /// A `SKILL.md` with `trust: approved` contributed via `extend`
    /// must appear in `list_quarantined()` (not `list()`), and the
    /// only way out is an explicit `approve()` call.
    #[tokio::test]
    async fn extension_contributed_skill_is_quarantined_regardless_of_frontmatter() {
        let tmp = TempDir::new("ext-quarantine");
        let bundle = tmp.path().join("bundle");
        fs::create_dir_all(&bundle).unwrap();
        // Frontmatter explicitly asks for approved — the trust matrix
        // must ignore it for `extend(...)` (R-skills-3 row 3).
        write(&bundle, "SKILL.md", &approved_skill_md("starter.ext.greet"));

        let registry = SkillRegistry::builder()
            .with_approval_store(InMemoryApprovalStore::new())
            .extend(vec![ContributedSkill::new(&bundle)])
            .build()
            .await
            .expect("build ok");

        // It is *only* in the quarantined list.
        let q_ids: Vec<String> = registry
            .list_quarantined()
            .iter()
            .map(|s| s.id.to_string())
            .collect();
        assert_eq!(q_ids, vec!["starter.ext.greet".to_string()]);
        assert!(registry.list().is_empty(), "must not be approved");

        // get() still finds it (operator UI path).
        let skill_id = SkillId::new("starter.ext.greet").unwrap();
        let found = registry.get(&skill_id).expect("get sees quarantined");
        assert_eq!(found.trust, Trust::Quarantined);
        let hash = found.bundle_hash.clone();

        // Approve flips the partition; no reload needed.
        registry
            .approve(&skill_id, &hash, &principal())
            .await
            .expect("approve ok");

        assert!(registry.list_quarantined().is_empty());
        let a_ids: Vec<String> = registry.list().iter().map(|s| s.id.to_string()).collect();
        assert_eq!(a_ids, vec!["starter.ext.greet".to_string()]);
        assert_eq!(
            registry.get(&skill_id).unwrap().trust,
            Trust::Approved,
            "approved skill must reflect trust"
        );
    }

    /// Smoke 2: "Hash mismatch re-quarantines."
    ///
    /// Approve at hash H1, edit the bundle, reload. The new hash H2
    /// has no row, so the matrix re-quarantines. The H1 row stays in
    /// `ApprovalStore::list()` (inert — drift never mutates the
    /// store, R-skills-7).
    #[tokio::test]
    async fn hash_mismatch_re_quarantines_and_keeps_prior_row_inert() {
        let tmp = TempDir::new("hash-drift");
        let bundle = tmp.path().join("bundle");
        fs::create_dir_all(&bundle).unwrap();
        // Frontmatter asks for quarantined so the matrix's
        // approval-store promotion path is exercised (the
        // approved-by-default path would short-circuit it).
        write(
            &bundle,
            "SKILL.md",
            &quarantined_skill_md("starter.drift.x"),
        );

        let store = Arc::new(InMemoryApprovalStore::new());
        let registry = SkillRegistry::builder()
            .with_approval_store_arc(store.clone() as Arc<dyn ApprovalStore>)
            .load_dir(tmp.path())
            .build()
            .await
            .expect("build ok");

        let skill_id = SkillId::new("starter.drift.x").unwrap();
        let h1 = registry.get(&skill_id).unwrap().bundle_hash.clone();
        assert_eq!(registry.list_quarantined().len(), 1);

        registry
            .approve(&skill_id, &h1, &principal())
            .await
            .expect("approve ok");
        assert_eq!(registry.list().len(), 1);
        assert!(registry.list_quarantined().is_empty());

        // Edit the body — bundle hash changes.
        write(
            &bundle,
            "SKILL.md",
            &(quarantined_skill_md("starter.drift.x") + "\nADDED LINE\n"),
        );

        registry.reload().await.expect("reload ok");

        let h2 = registry.get(&skill_id).unwrap().bundle_hash.clone();
        assert_ne!(h1, h2, "bundle hash must change after edit");

        // Re-quarantined: not in approved, present in quarantined.
        assert!(
            registry.list().is_empty(),
            "no approval row matches H2 — must re-quarantine"
        );
        assert_eq!(registry.list_quarantined().len(), 1);
        assert_eq!(registry.get(&skill_id).unwrap().trust, Trust::Quarantined);

        // The H1 row is **still in the store** (inert) — R-skills-7:
        // drift never mutates the store. Only an explicit revoke
        // can remove it.
        let rows = store.list().await.unwrap();
        assert_eq!(rows.len(), 1, "drift must not mutate the store");
        assert_eq!(rows[0].bundle_hash, h1);
        // Lookup of the *new* hash returns None (the row is keyed on
        // the old hash); selectors will see quarantined.
        assert!(store.lookup(&skill_id, &h2).await.unwrap().is_none());
    }

    /// Duplicate skill ids across sources must fail loudly so an
    /// extension typo cannot silently shadow a first-party bundle.
    #[tokio::test]
    async fn duplicate_skill_ids_are_rejected() {
        let tmp = TempDir::new("dup");
        let a = tmp.path().join("a");
        fs::create_dir_all(&a).unwrap();
        write(&a, "SKILL.md", &approved_skill_md("starter.dup.x"));

        let b = tmp.path().join("b");
        fs::create_dir_all(&b).unwrap();
        write(&b, "SKILL.md", &approved_skill_md("starter.dup.x"));

        let err = SkillRegistry::builder()
            .with_approval_store(InMemoryApprovalStore::new())
            .load_dir(tmp.path())
            .build()
            .await
            .expect_err("duplicate must fail");
        assert!(matches!(err, LoadError::DuplicateSkillId { .. }));
    }

    /// `load_dir` honours `trust: quarantined` from the frontmatter
    /// (R-skills-3 row 2).
    #[tokio::test]
    async fn load_dir_honours_frontmatter_quarantined() {
        let tmp = TempDir::new("fm-quar");
        let bundle = tmp.path().join("bundle");
        fs::create_dir_all(&bundle).unwrap();
        write(&bundle, "SKILL.md", &quarantined_skill_md("starter.fm.q"));

        let registry = SkillRegistry::builder()
            .with_approval_store(InMemoryApprovalStore::new())
            .load_dir(tmp.path())
            .build()
            .await
            .expect("build ok");
        assert_eq!(registry.list_quarantined().len(), 1);
        assert!(registry.list().is_empty());
    }

    /// `load_dir_quarantined` ignores `trust: approved`
    /// (R-skills-3: staging directories).
    #[tokio::test]
    async fn load_dir_quarantined_overrides_frontmatter_approved() {
        let tmp = TempDir::new("ldq");
        let bundle = tmp.path().join("bundle");
        fs::create_dir_all(&bundle).unwrap();
        write(&bundle, "SKILL.md", &approved_skill_md("starter.ldq.x"));

        let registry = SkillRegistry::builder()
            .with_approval_store(InMemoryApprovalStore::new())
            .load_dir_quarantined(tmp.path())
            .build()
            .await
            .expect("build ok");
        assert_eq!(registry.list_quarantined().len(), 1);
        assert!(registry.list().is_empty());
    }
}
