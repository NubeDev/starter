//! Skill-selection re-export.
//!
//! Per `DOCS/flow/scope/SCOPE.md` "Skills bind to the `ai-agent` node
//! kind": skill selection runs once per outer flow run and threads
//! through every `ai-agent` node in the flow as a `SkillSelection`.
//!
//! The canonical `SkillSelection` type lives in `starter-skills`. That
//! crate is **not yet a workspace member** (per the agent SCOPE its
//! addition is planned but unscheduled). Until it ships the `skills`
//! cargo feature is a declared-but-inert placeholder: the re-export
//! below is gated on both `feature = "skills"` and the impossible
//! `cfg(any())` so `--all-features` (the Phase 1 stage-7 smoke) still
//! builds green. When `starter-skills` lands as a workspace member,
//! drop the `any()` gate and add the matching optional dependency in
//! `Cargo.toml`.

#[cfg(all(feature = "skills", any()))]
pub use starter_skills::SkillSelection;
