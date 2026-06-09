//! The agent capability: drive a coding agent (Claude Code, Codex, Gemini CLI…)
//! over a task. Tier 2 of the facade.
//!
//! The `Agent` trait is always defined so the surface is stable; the zag-backed
//! implementation is behind the `agent` feature. Where inference is "messages ->
//! completion", an agent run is "task -> session that edits a repo", so this is a
//! deliberately different shape rather than a forced fit into `Inference`.

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::event::Event;
use crate::model::ModelRef;

/// A unit of agent work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Which agent backend to drive, e.g. `"claude"`, `"codex"`, `"gemini"`.
    pub backend: String,
    /// The instruction given to the agent.
    pub prompt: String,
    /// Optional model override; falls back to the backend's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    /// Working directory the agent operates in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Run in an isolated git worktree (zag supports this natively).
    #[serde(default)]
    pub isolate_worktree: bool,
}

impl AgentTask {
    pub fn new(backend: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            prompt: prompt.into(),
            model: None,
            cwd: None,
            isolate_worktree: false,
        }
    }
}

/// The outcome of a finished agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutcome {
    pub text: String,
    /// Session id, for resumption via the backend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Tier-2 capability: run an agent task, blocking or streamed.
#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&self, task: AgentTask) -> Result<AgentOutcome>;

    async fn run_stream(
        &self,
        task: AgentTask,
    ) -> Result<BoxStream<'static, Result<Event>>>;
}

#[cfg(feature = "agent")]
pub use zag_impl::ZagAgent;

#[cfg(feature = "agent")]
mod zag_impl {
    use super::*;
    use crate::error::Error;
    use crate::model::AliasMap;

    /// zag-backed [`Agent`]. The actual call into zag's `AgentBuilder` is isolated
    /// to the two methods below so a zag API change touches only this adapter.
    pub struct ZagAgent {
        aliases: AliasMap,
    }

    impl ZagAgent {
        pub fn new(aliases: AliasMap) -> Self {
            Self { aliases }
        }
    }

    #[async_trait]
    impl Agent for ZagAgent {
        async fn run(&self, task: AgentTask) -> Result<AgentOutcome> {
            let _model = task
                .model
                .as_ref()
                .map(|m| self.aliases.resolve(m));
            // ADAPTER BOUNDARY: translate `task` into a zag AgentBuilder, exec it,
            // and map the result back. Pending verification of zag's exact 0.x API
            // surface before wiring the concrete calls.
            let _ = &task;
            Err(Error::Unsupported("zag agent run not yet wired"))
        }

        async fn run_stream(
            &self,
            task: AgentTask,
        ) -> Result<BoxStream<'static, Result<Event>>> {
            let _ = &task;
            Err(Error::Unsupported("zag agent stream not yet wired"))
        }
    }
}
