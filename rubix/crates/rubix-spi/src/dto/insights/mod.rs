//! Insights goal — REST DTOs + tool descriptors.
//!
//! One file per verb. The insights family today wires four verbs
//! (`rubix.insights.rule.{list, create, enable, disable}`) backed
//! by an in-memory store; a PG-backed adapter is a tracked
//! follow-up.

pub mod rule_create;
pub mod rule_list;
pub mod rule_toggle;
