//! Skill-bundle identifiers shipped alongside `starter-insights`
//! (Insights SCOPE R-ins-5).
//!
//! The three meta agents live as static skill bundles under
//! `skills/starter.insights.{rule-author,explain,tuner}/SKILL.md`
//! per agent SCOPE R4 (skills are static metadata; the loader
//! quarantines new bundles by content hash and approves on first
//! sight per agent R4).
//!
//! This module re-exports the canonical bundle ids so callers
//! (`SkillSelection::with_bundle`, audit logs, the explainer
//! agent's bundle-pin check) reference one source of truth.

/// `starter.insights.rule-author` — proposes a `rule.sql` /
/// `rule.rhai` body from a schema + sample rows. Drafts only;
/// promotion goes through the approval flow per agent R4.
pub const BUNDLE_RULE_AUTHOR: &str = "starter.insights.rule-author";

/// `starter.insights.explain` — narrates a `Verdict` in plain
/// language given the window of data that produced it. Output is a
/// slot value, not a side effect.
pub const BUNDLE_EXPLAIN: &str = "starter.insights.explain";

/// `starter.insights.tuner` — reads false-positive / false-negative
/// feedback and proposes threshold deltas as drafts. Never
/// auto-applies; gated by approval.
pub const BUNDLE_TUNER: &str = "starter.insights.tuner";

/// Every insights skill bundle id, in canonical order.
pub const ALL: &[&str] = &[BUNDLE_RULE_AUTHOR, BUNDLE_EXPLAIN, BUNDLE_TUNER];
