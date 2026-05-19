//! # starter-flow-nodes
//!
//! Built-in node kinds the engine ships with: `ai-agent`, `tool-call`,
//! `transform`, `branch`, `merge`, `gate`, `subflow`,
//! `trigger.{explicit, event, schedule, webhook}`, `http-out`, `log`,
//! `sleep`. Each behind its own cargo feature so a consumer only pays
//! for the kinds it uses.
//!
//! Phase 1 of `DOCS/flow/scope/SCOPE.md` ships this crate as an empty
//! skeleton. Each module file declares only its [R10](../DOCS/flow/scope/SCOPE.md)
//! reverse-DNS `KIND_ID` constant in the reserved `starter.flow.*`
//! namespace. `NodeBehavior` impls land in Phase 4 (`ai-agent`, D1
//! resolution) and Phase 5 (the remainder); `transform` and `tool-call`
//! follow once the Phase 2 engine has somewhere to register them.
//!
//! The `all-kinds` aggregate feature enables every built-in kind at
//! once — used by `cargo check --features=all-kinds` as the skeleton's
//! green-build gate and by downstream test crates that want the full
//! set.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "transform")]
pub mod transform;

#[cfg(feature = "tool-call")]
pub mod tool_call;

#[cfg(feature = "ai-agent")]
pub mod ai_agent;

#[cfg(feature = "branch")]
pub mod branch;

#[cfg(feature = "merge")]
pub mod merge;

#[cfg(feature = "gate")]
pub mod gate;

#[cfg(feature = "subflow")]
pub mod subflow;

#[cfg(feature = "trigger-explicit")]
pub mod trigger_explicit;

#[cfg(feature = "trigger-event")]
pub mod trigger_event;

#[cfg(feature = "trigger-schedule")]
pub mod trigger_schedule;

#[cfg(feature = "trigger-webhook")]
pub mod trigger_webhook;

#[cfg(feature = "http-out")]
pub mod http_out;

#[cfg(feature = "log")]
pub mod log;

#[cfg(feature = "sleep")]
pub mod sleep;
