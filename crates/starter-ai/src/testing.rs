//! Test-only fakes for the AI runner trait — gated behind the
//! `testing` cargo feature so smoke tests opt in explicitly.
//!
//! Per Phase 4 D-F4.11: this is where `RecordingAiRunner` lives. The
//! body re-uses the same shape the Phase 4 stage-4 `ai-agent` body
//! unit tests prototyped inline; promoting it here lets the Phase 4
//! SCOPE smokes share one canonical recording fake instead of each
//! cloning the boilerplate.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use starter_spi::ai::{
    AiRunner, Cancel, Event, Provider, RunResult, RunnerError, RunnerInput, SessionId, ToolUse,
};

/// One scripted turn the [`RecordingAiRunner`] replays.
///
/// `text` becomes the assistant's final-turn text; `tool_uses` are
/// emitted to the engine. If `tool_uses` is empty the loop treats the
/// turn as terminal; otherwise the body dispatches the tool calls and
/// invokes the runner again with the next scripted turn.
#[derive(Clone, Debug, Default)]
pub struct ScriptTurn {
    /// Assistant text for this turn.
    pub text: String,
    /// Tool calls the model emits this turn.
    pub tool_uses: Vec<ToolUse>,
}

impl ScriptTurn {
    /// Construct a terminal text-only turn.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            tool_uses: Vec::new(),
        }
    }

    /// Construct a tool-only turn.
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self {
            text: String::new(),
            tool_uses: vec![ToolUse {
                id: id.into(),
                name: name.into(),
                input,
            }],
        }
    }
}

/// One recorded `AiRunner::run` invocation.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RecordedCall {
    /// Number of history messages handed to the runner this turn.
    pub history_len: usize,
    /// Number of tool definitions advertised to the model this turn.
    pub tools_count: usize,
    /// The `SessionId` the runner was called with.
    pub session_id: SessionId,
}

#[derive(Default)]
struct Inner {
    script: Vec<ScriptTurn>,
    cursor: usize,
    calls: Vec<RecordedCall>,
}

/// `AiRunner` that records every call and replays a configurable
/// script of `(text, tool_calls)` per turn. Phase 4 smokes use it
/// to assert R1 + R2 + R12 invariants without spinning real
/// provider impls.
pub struct RecordingAiRunner {
    provider: Provider,
    inner: Mutex<Inner>,
}

impl RecordingAiRunner {
    /// Construct a recording runner over the given script. The
    /// provider tag defaults to [`Provider::Anthropic`] — change
    /// via [`Self::with_provider`] if a smoke needs a specific tag.
    pub fn new(script: Vec<ScriptTurn>) -> Arc<Self> {
        Arc::new(Self {
            provider: Provider::Anthropic,
            inner: Mutex::new(Inner {
                script,
                cursor: 0,
                calls: Vec::new(),
            }),
        })
    }

    /// Override the recorded provider tag.
    pub fn with_provider(mut self: Arc<Self>, provider: Provider) -> Arc<Self> {
        // Safety: only one Arc exists at construction.
        let m = Arc::get_mut(&mut self).expect("RecordingAiRunner is uniquely owned at this point");
        m.provider = provider;
        self
    }

    /// Snapshot the list of calls recorded so far.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.inner.lock().unwrap().calls.clone()
    }
}

#[async_trait]
impl AiRunner for RecordingAiRunner {
    fn provider(&self) -> &Provider {
        &self.provider
    }

    async fn ready(&self) -> bool {
        true
    }

    async fn run(
        &self,
        input: RunnerInput,
        session_id: SessionId,
        _on_event: mpsc::Sender<Event>,
        _cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError> {
        let (history_len, tools_count) = match &input {
            RunnerInput::Rest(c) => (c.history.len(), c.tools.len()),
            RunnerInput::Cli(_) => (0, 0),
        };
        let turn_script = {
            let mut g = self.inner.lock().unwrap();
            g.calls.push(RecordedCall {
                history_len,
                tools_count,
                session_id,
            });
            let idx = g.cursor;
            g.cursor += 1;
            g.script.get(idx).cloned().unwrap_or_else(|| ScriptTurn {
                text: "(no more scripted turns)".to_string(),
                ..Default::default()
            })
        };

        Ok(RunResult {
            text: turn_script.text,
            tool_uses: turn_script.tool_uses,
            provider: self.provider.to_string(),
            ..RunResult::default()
        })
    }
}
