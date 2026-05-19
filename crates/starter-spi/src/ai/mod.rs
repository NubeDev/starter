//! AI-runner seam. Concrete provider impls live in `starter-ai`; this
//! module defines the trait + associated types every provider speaks.
//!
//! Types and trait shape are lifted from `codeless-workspace/ai-runner`
//! per SCOPE q7. The `Cancel` indirection is the only deviation —
//! starter-spi must not depend on `tokio_util`.

mod event;
mod input;
mod provider;
mod result;
mod runner;
mod session;

pub use event::{Event, EventKind};
pub use input::{
    CliCfg, HistoryMessage, PermissionMode, RestCfg, RunnerInput, ToolChoice, ToolDef,
};
pub use provider::Provider;
pub use result::{RunResult, RunnerError, ToolCallEntry, ToolUse};
pub use runner::{AiRunner, Cancel, OnEvent};
pub use session::SessionId;
