//! # starter-flow-nodes
//!
//! Built-in node kinds the engine ships with: `ai-agent`, `tool-call`,
//! `transform`, `branch`, `merge`, `gate`, `subflow`,
//! `trigger.{explicit, event, schedule, webhook}`, `http-out`, `log`,
//! `sleep`. Each behind its own cargo feature so a consumer only pays
//! for the kinds it uses.
//!
//! Phase 1 of `DOCS/flow/scope/SCOPE.md` ships this crate as an empty
//! skeleton. `transform` and `tool-call` land in Phase 2; `ai-agent`
//! in Phase 4 (D1 resolution); the rest in Phase 5.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
