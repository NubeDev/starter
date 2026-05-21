//! `starter-ext-flow` — the adapter that surfaces extension-contributed
//! flow artefacts into the host's flow runtime.
//!
//! Per [`DOCS/agent/SCOPE.md`] R-agent-4 the same adapter handles three
//! `contributes:` branches on a `block.yaml` manifest:
//!
//! - `contributes.flows`  — extension-shipped flow YAML files (later
//!   phase of the flow track; not wired here).
//! - `contributes.skills` — extension-shipped `SKILL.md` bundle
//!   directories (DOCS/agent/SKILLS.md R-agent-4 + Phase 6).
//! - `contributes.nodes`  — extension-supplied flow node kinds
//!   (later phase; not wired here).
//!
//! This crate is intentionally narrow today: Phase 6 of the
//! `starter-skills` job wires only `contributes.skills`. The other
//! two branches land in their respective tracks alongside this same
//! adapter — one crate, one wire format
//! (DOCS/agent/SKILLS.md §"Relationship to existing crates").
//!
//! ## Trust matrix
//!
//! Skills surfaced through this adapter feed
//! [`starter_skills::SkillRegistry::extend`], which classifies
//! everything it receives as
//! [`Trust::Quarantined`][starter_skills::Trust::Quarantined]
//! regardless of the bundle's frontmatter `trust:` field
//! (DOCS/agent/SKILLS.md R-skills-3 row 3). An extension cannot
//! ship pre-approved skills; the operator must approve each
//! `(skill_id, bundle_hash)` explicitly through
//! [`SkillRegistry::approve`][starter_skills::SkillRegistry::approve].
//!
//! ## What this crate does **not** do
//!
//! - It does not load the extension manifest itself. That is
//!   `starter-ext-host`'s job; this adapter consumes the parsed
//!   [`Manifest`][starter_ext_spi::manifest::Manifest].
//! - It does not parse `SKILL.md`. That is `starter-skills`' job;
//!   this adapter only enumerates bundle directories and hands
//!   their paths to [`SkillRegistry::extend`].
//! - It does not build the [`SkillRegistry`] for the host. The
//!   host owns the registry lifecycle and reload cadence
//!   (R-skills-8); this adapter contributes a batch into a
//!   pre-existing builder or onto an already-built registry's
//!   next reload.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use starter_ext_spi::manifest::Manifest;
use starter_skills::ContributedSkill;

/// Errors the [`contributed_skills`] walker can return.
///
/// Every variant carries the offending path so an operator can find
/// and fix the bundle without grepping logs.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContributeSkillsError {
    /// A `contributes.skills[].dir` resolved to a path that is not a
    /// readable directory. Distinct from "directory exists but
    /// contains no bundles": the latter is **not** an error (an
    /// extension may legitimately ship zero skills under a declared
    /// directory while it iterates on them).
    #[error("contributes.skills[].dir {dir} is not a readable directory: {source}")]
    InvalidSkillsDir {
        /// The resolved (`extension_root + dir`) path that failed.
        dir: PathBuf,
        /// Underlying I/O error from `read_dir`.
        #[source]
        source: std::io::Error,
    },
}

/// Walk every `contributes.skills[].dir` declared on `manifest` and
/// return the [`ContributedSkill`] values the host should feed into
/// [`SkillRegistry::extend`][starter_skills::SkillRegistry::extend].
///
/// Each `dir` is resolved relative to `extension_root` (the directory
/// the extension's `block.yaml` lives in). Inside each resolved
/// directory the adapter looks for **one level** of sub-directories
/// that contain a `SKILL.md` file — the same shape
/// [`SkillRegistry::load_dir`][starter_skills::SkillRegistry::builder]
/// uses for the host's own skills tree. Sub-directories without a
/// `SKILL.md` are ignored so a stray `node_modules/` cannot accidentally
/// register a bundle.
///
/// Bundle directories are returned in deterministic order (sorted by
/// path) so downstream errors like
/// [`LoadError::DuplicateSkillId`][starter_skills::LoadError::DuplicateSkillId]
/// reproduce across runs.
///
/// **Trust:** every entry returned here will land
/// [`Trust::Quarantined`][starter_skills::Trust::Quarantined] when
/// passed to [`SkillRegistry::extend`][starter_skills::SkillRegistry::extend]
/// regardless of the bundle's frontmatter `trust:` field
/// (R-skills-3 row 3).
pub fn contributed_skills(
    manifest: &Manifest,
    extension_root: &Path,
) -> Result<Vec<ContributedSkill>, ContributeSkillsError> {
    let mut out: Vec<ContributedSkill> = Vec::new();
    for entry in &manifest.contributes.skills {
        let resolved = extension_root.join(&entry.dir);
        let read = std::fs::read_dir(&resolved).map_err(|source| {
            ContributeSkillsError::InvalidSkillsDir {
                dir: resolved.clone(),
                source,
            }
        })?;
        let mut bundle_dirs: Vec<PathBuf> = Vec::new();
        for dirent in read {
            let dirent = dirent.map_err(|source| ContributeSkillsError::InvalidSkillsDir {
                dir: resolved.clone(),
                source,
            })?;
            let file_type =
                dirent
                    .file_type()
                    .map_err(|source| ContributeSkillsError::InvalidSkillsDir {
                        dir: resolved.clone(),
                        source,
                    })?;
            if !file_type.is_dir() {
                continue;
            }
            let candidate = dirent.path();
            if candidate.join("SKILL.md").is_file() {
                bundle_dirs.push(candidate);
            }
        }
        bundle_dirs.sort();
        tracing::debug!(
            target: "starter_ext_flow::skills",
            extension_id = %manifest.id.as_str(),
            dir = %resolved.display(),
            count = bundle_dirs.len(),
            "discovered contributes.skills bundles (will land quarantined)"
        );
        for dir in bundle_dirs {
            out.push(ContributedSkill::new(dir));
        }
    }
    Ok(out)
}
