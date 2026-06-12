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

    async fn run_stream(&self, task: AgentTask) -> Result<BoxStream<'static, Result<Event>>>;
}

#[cfg(feature = "agent")]
pub use zag_impl::ZagAgent;

#[cfg(feature = "agent")]
mod zag_impl {
    use super::*;
    use crate::error::Error;
    use crate::event::Event;
    use crate::model::{AliasMap, ModelRef, Size};
    use futures::StreamExt;
    use zag::builder::AgentBuilder;

    /// Translate a [`ModelRef`] into the model string zag's CLI wrappers accept.
    /// A size alias maps to zag's own size words (`small`/`medium`/`large`) — NOT
    /// to a provider API id, which zag rejects. A concrete id is passed verbatim
    /// (the caller asserts zag/the CLI accepts it). `None` means "let zag pick its
    /// default" rather than forcing a possibly-invalid value.
    fn zag_model(m: &ModelRef) -> Option<&'static str> {
        match m {
            ModelRef::Alias(Size::Small) => Some("small"),
            ModelRef::Alias(Size::Medium) => Some("medium"),
            ModelRef::Alias(Size::Large) => Some("large"),
            // A concrete id is handled by the caller (passed verbatim); this
            // helper only resolves the size aliases to zag's vocabulary.
            ModelRef::Concrete(_) => None,
        }
    }

    /// zag-backed [`Agent`]. All calls into zag's `AgentBuilder` are isolated to
    /// this adapter so a zag API change touches only this file.
    pub struct ZagAgent {
        #[allow(dead_code)] // retained for parity with the inference tier; zag
        // resolves size words itself, so the alias map isn't consulted here.
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
            // `output_format("json")` is required for `exec` to capture the
            // agent's reply into `AgentOutput.result`: with the default format
            // zag's non-streaming exec returns an empty result for the Claude
            // provider. JSON mode makes the final result line parseable.
            let mut b = AgentBuilder::new()
                .provider(&task.backend)
                .auto_approve(true)
                .output_format("json");
            if let Some(cwd) = task.cwd.as_deref() {
                b = b.root(cwd);
            }
            match task.model.as_ref() {
                // A size alias maps to zag's own size words (small/medium/large) —
                // NOT a provider API id, which zag's CLI wrappers reject.
                Some(m @ ModelRef::Alias(_)) => {
                    if let Some(tier) = zag_model(m) {
                        b = b.model(tier);
                    }
                }
                // A concrete id is the caller's explicit choice — pass it verbatim.
                Some(ModelRef::Concrete(id)) => {
                    b = b.model(id);
                }
                None => {}
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

        async fn run_stream(&self, task: AgentTask) -> Result<BoxStream<'static, Result<Event>>> {
            // zag's `exec_streaming` is built for the interactive TUI (it waits on
            // a stdin turn pipe) and does not yield events when driven one-shot
            // from a library, so we use the reliable batch `exec` for the reply.
            // To avoid a long blank wait, we emit an immediate `Progress` event so
            // the UI shows the agent is working, then run, then a terminal `Done`.
            // (Token-level streaming would require zag to expose a one-shot
            // streaming consumer; tracked separately.)
            // Build the run future before the stream so the model call starts as
            // soon as it's polled.
            let builder = self.builder(&task);
            let prompt = task.prompt.clone();
            let run = async move {
                builder
                    .exec(&prompt)
                    .await
                    .map(|o| o.result.unwrap_or_default())
                    .map_err(|e| Error::Provider(e.to_string()))
            };

            // A tiny state machine: yield Progress immediately, then await the run
            // and yield Done (or an error), then end.
            type RunFut =
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>;
            enum Phase {
                Start(RunFut),
                Running(RunFut),
                End,
            }
            let stream = futures::stream::unfold(Phase::Start(Box::pin(run)), |phase| async move {
                match phase {
                    Phase::Start(fut) => {
                        let progress = Event::Progress {
                            message: "Working…".to_string(),
                        };
                        Some((Ok(progress), Phase::Running(fut)))
                    }
                    Phase::Running(fut) => match fut.await {
                        Ok(text) => Some((Ok(Event::Done { text, usage: None }), Phase::End)),
                        Err(e) => Some((Err(e), Phase::End)),
                    },
                    Phase::End => None,
                }
            });
            Ok(stream.boxed())
        }
    }
}
