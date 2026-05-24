//! Test doubles. Not gated behind `cfg(test)` so downstream crates
//! (e.g. starter-flow-node-loop's integration test) can use them too.

use std::sync::Mutex;

use async_trait::async_trait;
use starter_spi::ai::{
    AiRunner, Cancel, OnEvent, Provider, RunResult, RunnerError, RunnerInput, SessionId,
};

/// A mock [`AiRunner`] that returns canned [`RunResult`]s in order
/// per call.
pub struct MockAiRunner {
    provider: Provider,
    scripted: Mutex<Vec<RunResult>>,
}

impl MockAiRunner {
    /// Build a mock that returns `scripted[i]` on the i-th `run` call.
    pub fn new(scripted: Vec<RunResult>) -> Self {
        Self {
            provider: Provider::Claude,
            scripted: Mutex::new(scripted.into_iter().rev().collect()),
        }
    }
}

#[async_trait]
impl AiRunner for MockAiRunner {
    fn provider(&self) -> &Provider {
        &self.provider
    }
    async fn ready(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _input: RunnerInput,
        _session: SessionId,
        _events: OnEvent,
        _cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError> {
        let next = self
            .scripted
            .lock()
            .expect("scripted lock not poisoned")
            .pop()
            .expect("MockAiRunner exhausted: more `run` calls than scripted results");
        Ok(next)
    }
}
