//! Skill-selection re-export.
//!
//! Per `DOCS/flow/scope/SCOPE.md` "Skills bind to the `ai-agent` node
//! kind": skill selection runs once per outer flow run and threads
//! through every `ai-agent` node in the flow as a `SkillSelection`.
//!
//! The canonical `SkillSelection` type lives in `starter-skills`. That
//! crate is **not yet a workspace member** (per the agent SCOPE its
//! addition is planned but unscheduled). The re-export below is
//! feature-gated behind a default-off `skills` feature so the workspace
//! still builds today; the gate flips on once `starter-skills` ships
//! and is added as a workspace dependency.

#[cfg(feature = "skills")]
pub use starter_skills::SkillSelection;
