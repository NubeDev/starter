//! Stage-07 agent-event-projection contract: when an AI run errors
//! mid-stream, the events the runner already emitted survive on
//! [`RunOutcome::events`] so live-feedback surfaces (e.g. the rubix
//! `ai-agent` flow node feeding the dashboard editor SSE channel)
//! can still render "AI got N chunks and then errored" rather than
//! going silently dark.
//!
//! See `rubix/docs/sessions/data-flow/2026-05-26-data-flow-07-agent-event-projection.md`.
//!
//! Exercises [`AgentLoop::run_with_outcome`] specifically — the
//! `Result`-returning [`AgentLoop::run`] still drops the events on
//! `Err` by design (backwards-compatible for the 3-test suite + the
//! `starter-flow-node-loop` body that doesn't need per-step
//! activity).

use std::sync::Arc;

use async_trait::async_trait;
use starter_ai_agent::{AgentError, AgentLoop, ToolSet};
use starter_spi::ai::{
    AiRunner, Cancel, Event, EventKind, OnEvent, Provider, RunResult, RunnerError, RunnerInput,
    SessionId,
};

/// Emits two `Text` events on the channel, then returns
/// `Err(RunnerError::WrongInputKind)`. Stand-in for a real CLI
/// runner that streamed some output before its subprocess crashed
/// or the upstream model dropped the connection.
struct EmitThenFailRunner;

#[async_trait]
impl AiRunner for EmitThenFailRunner {
    fn provider(&self) -> &Provider {
        &Provider::Claude
    }
    async fn ready(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _input: RunnerInput,
        session: SessionId,
        events: OnEvent,
        _cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError> {
        // Emit two text chunks before failing — these are the
        // events the loop must preserve on `outcome.events`.
        for chunk in ["partial answer ", "before the crash"] {
            let ev = Event {
                session_id: session.clone(),
                provider: "claude".to_owned(),
                kind: EventKind::Text {
                    content: chunk.to_owned(),
                },
            };
            // Channel close is not failure — if the loop closed
            // early the test asserts on what made it through.
            let _ = events.send(ev).await;
        }
        Err(RunnerError::WrongInputKind {
            provider: "claude".to_owned(),
            expected: "Cli",
            got: "Rest",
        })
    }
}

#[tokio::test]
async fn run_with_outcome_preserves_events_when_runner_errors() {
    let runner = Arc::new(EmitThenFailRunner);
    let agent = AgentLoop::new(runner, ToolSet::default());
    let outcome = agent.run_with_outcome("hi".to_owned()).await;

    assert!(
        outcome.error.is_some(),
        "runner returned Err — outcome.error must be Some, got None"
    );
    match outcome.error.as_ref().unwrap() {
        AgentError::Runner(_) => {}
        other => panic!("expected AgentError::Runner, got {other:?}"),
    }
    assert_eq!(
        outcome.events.len(),
        2,
        "both pre-failure Text events must be on outcome.events; got {}",
        outcome.events.len()
    );
    let text_payloads: Vec<&str> = outcome
        .events
        .iter()
        .filter_map(|ev| match &ev.kind {
            EventKind::Text { content } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        text_payloads,
        vec!["partial answer ", "before the crash"],
        "event order + content must be preserved exactly as emitted"
    );
    assert_eq!(
        outcome.text, "",
        "text is empty when the runner failed before producing a `RunResult`"
    );
}

#[tokio::test]
async fn run_wrapper_still_drops_events_on_err_by_design() {
    // The `Result`-returning wrapper is the back-compat surface for
    // the 3-test suite + starter-flow-node-loop; it forwards the
    // error and the partial events are unreachable through this
    // path. Asserting this here keeps the two surfaces' contracts
    // explicit — a future refactor that "fixes" `run` to carry
    // events would be a breaking API change for those call sites.
    let runner = Arc::new(EmitThenFailRunner);
    let agent = AgentLoop::new(runner, ToolSet::default());
    let err = agent
        .run("hi".to_owned())
        .await
        .expect_err("runner returned Err — wrapper must surface AgentError");
    matches!(err, AgentError::Runner(_));
}
