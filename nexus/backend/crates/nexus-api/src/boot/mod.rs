//! Boot-time operational infrastructure (WS-16).
//!
//! These modules are the operational *floor* under the control plane: a
//! [`runtime_canary`] that proves the tokio runtime is advancing (so a wedge is
//! visible via `/livez`), and a [`task_watchdog`] that wraps every long-lived
//! background task so its unexpected death is loud rather than inferred from the
//! absence of a log line. Neither changes product behaviour; both change how
//! fast an incident is diagnosed.
//!
//! Ported from the sibling `rubix` workspace
//! (`rubix-agent/src/boot/{runtime_canary,task_watchdog}.rs`), which earned them
//! the hard way after a runtime wedge that took hours to localise.

pub mod runtime_canary;
pub mod task_watchdog;
