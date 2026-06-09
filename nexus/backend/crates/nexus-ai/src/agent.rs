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
    use crate::event::Event;
    use crate::model::AliasMap;
    use futures::StreamExt;
    use zag::builder::AgentBuilder;

    /// zag-backed [`Agent`]. All calls into zag's `AgentBuilder` are isolated to
    /// this adapter so a zag API change touches only this file.
    pub struct ZagAgent {
        aliases: AliasMap,
    }

    impl ZagAgent {
        pub fn new(aliases: AliasMap) -> Self {
            Self { aliases }
        }

        /// Build a configured `AgentBuilder` for `task`. `auto_approve` is on
        /// because a control-plane run is non-interactive — there is no human at
        /// a TTY to approve tool calls.
        fn builder(&self, task: &AgentTask) -> AgentBuilder {
            let mut b = AgentBuilder::new()
                .provider(&task.backend)
                .auto_approve(true);
            if let Some(m) = task.model.as_ref() {
                // A size alias resolves to a concrete id; zag also accepts its own
                // tier names ("sonnet"), so a concrete passthrough covers both.
                b = b.model(&self.aliases.resolve(m));
            }
            b
        }
    }

    #[async_trait]
    impl Agent for ZagAgent {
        async fn run(&self, task: AgentTask) -> Result<AgentOutcome> {
            let output = self
                .builder(&task)
                .exec(&task.prompt)
                .await
                .map_err(|e| Error::Provider(e.to_string()))?;
            Ok(AgentOutcome {
                text: output.result.unwrap_or_default(),
                session_id: Some(output.session_id.to_string()),
            })
        }

        async fn run_stream(
            &self,
            task: AgentTask,
        ) -> Result<BoxStream<'static, Result<Event>>> {
            // zag streams live events through `on_log_event`, whose event schema
            // is not yet pinned in this adapter. Until those event fields are
            // mapped to the unified `Event`, the stream runs the task to
            // completion and emits a single terminal `Done` — correct and usable,
            // just not incremental. The broadcast/persist machinery in nexus-api
            // already treats `Done` as terminal, so consumers need no change when
            // incremental deltas are added here later.
            let outcome = self.run(task).await?;
            let done = Event::Done {
                text: outcome.text,
                usage: None,
            };
            Ok(futures::stream::once(async move { Ok(done) }).boxed())
        }
    }
}
