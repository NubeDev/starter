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
use starter_undo::{record_if_reversible, ChangeDraft, ReversibleRegistry, UndoCursor};

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

/// Task-local [`ActorSource`] backed by
/// [`starter_undo::actor_local`]. Reads the actor installed by the
/// transport (REST tools route, MCP server) for the duration of the
/// current dispatch; falls back to [`Actor::System`] when nothing
/// is installed so an unattributed dispatch still records an
/// audit-safe value rather than panicking.
#[derive(Clone, Default)]
pub struct LocalActor;

impl LocalActor {
    /// Construct.
    pub fn new() -> Self {
        Self
    }
}

impl ActorSource for LocalActor {
    fn actor(&self) -> Actor {
        starter_undo::actor_local::try_current().unwrap_or(Actor::System)
    }
}

/// Wraps a [`ReversibleTool`] so every successful invocation goes
/// through [`record_if_reversible`].
///
/// `T: ?Sized` so callers can pass either a concrete type (the
/// generic monomorphisation that integration tests use, e.g.
/// `UndoDispatcher<WidgetUpdateTool>`) or a trait object
/// (`Arc<dyn ReversibleTool>` from the boot-time registry builder
/// where a single `wrap_rev` helper handles every reversible
/// verb).
pub struct UndoDispatcher<T: ReversibleTool + ?Sized> {
    inner: Arc<T>,
    registry: Arc<ReversibleRegistry>,
    recorder: Arc<dyn ChangeRecorder>,
    actor: Arc<dyn ActorSource>,
    /// Optional redo cursor. When wired, a successful mutation
    /// (one that produces a `ChangeDraft` *and* whose kind is
    /// registered as reversible) clears the actor's redo stack.
    /// This pins proposal §3.4: re-doing across a fresh
    /// branch is not supported. Left `None` in unit tests that
    /// don't care about cursor invariants.
    cursor: Option<Arc<dyn UndoCursor>>,
}

impl<T: ReversibleTool + ?Sized> UndoDispatcher<T> {
    /// New dispatcher without cursor invalidation. Use this when
    /// the caller doesn't drive a redo stack (unit tests of
    /// individual reversible tools, integration tests of the
    /// record-and-replay path).
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
            cursor: None,
        }
    }

    /// New dispatcher that clears the actor's redo stack on each
    /// successful reversible mutation. The production boot path
    /// (`rubix_agent::registry::build_tool_registry`) uses this so
    /// the proposal §3.4 contract holds.
    pub fn with_cursor(
        inner: Arc<T>,
        registry: Arc<ReversibleRegistry>,
        recorder: Arc<dyn ChangeRecorder>,
        actor: Arc<dyn ActorSource>,
        cursor: Arc<dyn UndoCursor>,
    ) -> Self {
        Self {
            inner,
            registry,
            recorder,
            actor,
            cursor: Some(cursor),
        }
    }

    /// Invoke and return both the tool output and the recorded
    /// group id (when a draft was produced and the kind is
    /// registered). Useful for tests; production callers use the
    /// `Tool::invoke` path which discards the group id.
    pub async fn invoke_with_group(&self, input: Value) -> Result<(Value, Option<GroupId>)> {
        let actor = self.actor.actor();
        let output = self.inner.invoke(input.clone()).await?;
        let group = match self.inner.change_for(&input, &output) {
            Some(draft) => {
                record_if_reversible(&self.registry, &*self.recorder, actor.clone(), draft).await?
            }
            None => None,
        };
        // Per proposal §3.4: a new mutation by an actor clears
        // their redo stack. Only fire when the mutation actually
        // landed a changelog row — read-only or unregistered
        // verbs return `None` and must not invalidate the stack.
        if group.is_some() {
            if let Some(cursor) = self.cursor.as_ref() {
                cursor.clear_redo(&actor).await?;
            }
        }
        Ok((output, group))
    }
}

#[async_trait]
impl<T: ReversibleTool + ?Sized> Tool for UndoDispatcher<T> {
    fn definition(&self) -> ToolDefinition {
        self.inner.definition()
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let (output, _group) = self.invoke_with_group(input).await?;
        Ok(output)
    }
}
