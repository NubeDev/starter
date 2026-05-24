//! Undo-aware [`Tool`] dispatch wrapper.
//!
//! [`UndoDispatcher`] wraps any [`Tool`] whose effect can be
//! described as a [`ChangeDraft`] and records the draft through
//! [`starter_undo::record_if_reversible`] after a successful
//! [`Tool::invoke`]. Tools that do not produce a draft (read-only
//! verbs, status probes) pass through unchanged.
//!
//! The seam is the [`ReversibleTool`] trait — adapter implementations
//! sit next to the concrete tool and translate
//! `(input, output) → Option<ChangeDraft>`. This keeps undo wiring
//! out of the domain logic itself; see
//! [`docs/design/undo/`](../../../../docs/design/undo/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use starter_spi::changelog::{Actor, ChangeRecorder, GroupId};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result;
use starter_undo::{record_if_reversible, ChangeDraft, ReversibleRegistry};

/// Adapter that converts a tool invocation into an optional
/// [`ChangeDraft`]. Returning [`None`] tells the dispatcher the call
/// was a no-op for undo (e.g. a status read).
pub trait ReversibleTool: Tool {
    /// Inspect the original `input` and the successful `output`
    /// returned by [`Tool::invoke`] and decide what (if anything) to
    /// record for undo. Called only on the success path.
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft>;
}

/// Source of the [`Actor`] for the next dispatch. The agent loop
/// holds an [`Arc<dyn ActorSource>`] keyed on the current request
/// context — by the time a tool fires, the loop knows whether it is
/// running on behalf of a user, an agent run, or the system.
pub trait ActorSource: Send + Sync + 'static {
    /// Return the actor to stamp on the recorded change.
    fn actor(&self) -> Actor;
}

/// Convenience [`ActorSource`] that always returns the same actor.
/// Tests and single-tenant CLIs use this directly; the agent binary
/// uses a context-aware impl that consults the live request.
#[derive(Clone)]
pub struct StaticActor(pub Actor);

impl ActorSource for StaticActor {
    fn actor(&self) -> Actor {
        self.0.clone()
    }
}

/// Wraps a [`ReversibleTool`] so every successful invocation goes
/// through [`record_if_reversible`].
pub struct UndoDispatcher<T: ReversibleTool> {
    inner: Arc<T>,
    registry: Arc<ReversibleRegistry>,
    recorder: Arc<dyn ChangeRecorder>,
    actor: Arc<dyn ActorSource>,
}

impl<T: ReversibleTool> UndoDispatcher<T> {
    /// New dispatcher.
    pub fn new(
        inner: Arc<T>,
        registry: Arc<ReversibleRegistry>,
        recorder: Arc<dyn ChangeRecorder>,
        actor: Arc<dyn ActorSource>,
    ) -> Self {
        Self {
            inner,
            registry,
            recorder,
            actor,
        }
    }

    /// Invoke and return both the tool output and the recorded
    /// group id (when a draft was produced and the kind is
    /// registered). Useful for tests; production callers use the
    /// `Tool::invoke` path which discards the group id.
    pub async fn invoke_with_group(
        &self,
        input: Value,
    ) -> Result<(Value, Option<GroupId>)> {
        let output = self.inner.invoke(input.clone()).await?;
        let group = match self.inner.change_for(&input, &output) {
            Some(draft) => {
                record_if_reversible(
                    &self.registry,
                    &*self.recorder,
                    self.actor.actor(),
                    draft,
                )
                .await?
            }
            None => None,
        };
        Ok((output, group))
    }
}

#[async_trait]
impl<T: ReversibleTool> Tool for UndoDispatcher<T> {
    fn definition(&self) -> ToolDefinition {
        self.inner.definition()
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let (output, _group) = self.invoke_with_group(input).await?;
        Ok(output)
    }
}
