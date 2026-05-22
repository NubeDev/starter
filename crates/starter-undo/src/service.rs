//! Per-actor undo / redo cursor.
//!
//! Walk strategy: for an actor, the *next group to undo* is the
//! latest `group_id` whose rows were authored by that actor and is
//! not on the actor's redo stack. Undo loads the group, calls
//! [`Reversible::apply_inverse`] over its rows in reverse `at`
//! order, and pushes the group onto the redo stack. Redo pops from
//! the redo stack and calls [`Reversible::apply_forward`] in `at`
//! order.
//!
//! TODO(persistence): the redo stack lives in process memory keyed
//! by `(actor_kind, actor_id)`. Once a consumer needs cross-process
//! undo, replace [`InMemoryUndoCursor`] with a small
//! `starter_undo_cursors` table.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use starter_changelog::{ChangeFilter, ChangeLog};
use starter_spi::changelog::{Actor, Change, GroupId};
use starter_spi::{Error, Result};
use tokio::sync::Mutex;

use crate::registry::ReversibleRegistry;

/// Canonical string key for an [`Actor`] — used as the row key in
/// any [`UndoCursor`] backend.
///
/// - `User { subject }`  → `"user:{subject}"`
/// - `Agent { run_id, .. }` → `"agent:{run_id}"` (model deliberately
///   excluded so a re-attached run resumes its own stack)
/// - `System` → `"system:_"`
pub fn actor_key(actor: &Actor) -> String {
    match actor {
        Actor::User { subject } => format!("user:{subject}"),
        Actor::Agent { run_id, .. } => format!("agent:{run_id}"),
        Actor::System => "system:_".into(),
    }
}

/// Cursor backing — pluggable so a future SQL-backed cursor doesn't
/// require an [`UndoService`] rewrite.
#[async_trait]
pub trait UndoCursor: Send + Sync {
    /// Return the most recent group already undone for this actor
    /// (i.e. top of the redo stack), if any.
    async fn peek_redo(&self, actor: &Actor) -> Result<Option<GroupId>>;

    /// Push a group onto the actor's redo stack (called after a
    /// successful undo).
    async fn push_redo(&self, actor: &Actor, group: GroupId) -> Result<()>;

    /// Pop the actor's redo stack (called when redo succeeds).
    async fn pop_redo(&self, actor: &Actor) -> Result<Option<GroupId>>;

    /// Clear the actor's redo stack (called when a new mutation
    /// lands while the stack is non-empty — re-doing across a fresh
    /// branch is not supported).
    async fn clear_redo(&self, actor: &Actor) -> Result<()>;
}

/// In-process redo stack keyed by `(actor_kind, actor_id)`.
#[derive(Default)]
pub struct InMemoryUndoCursor {
    stacks: Mutex<HashMap<String, Vec<GroupId>>>,
}

impl InMemoryUndoCursor {
    /// Empty cursor.
    pub fn new() -> Self {
        Self::default()
    }

    fn key(actor: &Actor) -> String {
        actor_key(actor)
    }
}

#[async_trait]
impl UndoCursor for InMemoryUndoCursor {
    async fn peek_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let stacks = self.stacks.lock().await;
        Ok(stacks.get(&Self::key(actor)).and_then(|s| s.last().cloned()))
    }

    async fn push_redo(&self, actor: &Actor, group: GroupId) -> Result<()> {
        let mut stacks = self.stacks.lock().await;
        stacks.entry(Self::key(actor)).or_default().push(group);
        Ok(())
    }

    async fn pop_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let mut stacks = self.stacks.lock().await;
        Ok(stacks.get_mut(&Self::key(actor)).and_then(|s| s.pop()))
    }

    async fn clear_redo(&self, actor: &Actor) -> Result<()> {
        let mut stacks = self.stacks.lock().await;
        stacks.remove(&Self::key(actor));
        Ok(())
    }
}

/// Per-actor undo / redo over the changelog.
pub struct UndoService {
    log: Arc<dyn ChangeLog>,
    registry: Arc<ReversibleRegistry>,
    cursor: Arc<dyn UndoCursor>,
}

impl UndoService {
    /// Build with in-memory cursor.
    pub fn new(log: Arc<dyn ChangeLog>, registry: Arc<ReversibleRegistry>) -> Self {
        Self {
            log,
            registry,
            cursor: Arc::new(InMemoryUndoCursor::new()),
        }
    }

    /// Build with a custom cursor.
    pub fn with_cursor(
        log: Arc<dyn ChangeLog>,
        registry: Arc<ReversibleRegistry>,
        cursor: Arc<dyn UndoCursor>,
    ) -> Self {
        Self {
            log,
            registry,
            cursor,
        }
    }

    /// Undo the most recent group authored by `actor` that has not
    /// already been undone. Returns the group id that was undone.
    ///
    /// Errors:
    /// - [`Error::NotFound`] when the actor has no undoable group.
    /// - Whatever the [`Reversible`] impl returns (typically
    ///   [`Error::Conflict`] on stale `resource_version`).
    pub async fn undo(&self, actor: &Actor) -> Result<GroupId> {
        let already_undone = self.cursor.peek_redo(actor).await?;

        // Pull recent rows for this actor; pick the newest group
        // that is not already on the redo stack.
        let filter = actor_filter(actor);
        let page = self.log.list(&filter).await?;
        let target = page
            .items
            .iter()
            .map(|c| c.group_id.clone())
            .find(|g| Some(g) != already_undone.as_ref());

        let group = target.ok_or_else(|| Error::NotFound {
            what: format!("undo target for {actor:?}"),
        })?;

        let mut rows = self.log.group(&group).await?;
        rows.sort_by(|a, b| b.at.cmp(&a.at).then(b.id.0.cmp(&a.id.0)));

        for ch in &rows {
            self.dispatch_inverse(ch).await?;
        }

        self.cursor.push_redo(actor, group.clone()).await?;
        Ok(group)
    }

    /// Redo the most recently undone group for `actor`.
    pub async fn redo(&self, actor: &Actor) -> Result<GroupId> {
        let group = self.cursor.pop_redo(actor).await?.ok_or_else(|| {
            Error::NotFound {
                what: format!("redo target for {actor:?}"),
            }
        })?;

        let mut rows = self.log.group(&group).await?;
        rows.sort_by(|a, b| a.at.cmp(&b.at).then(a.id.0.cmp(&b.id.0)));

        for ch in &rows {
            self.dispatch_forward(ch).await?;
        }

        Ok(group)
    }

    async fn dispatch_inverse(&self, ch: &Change) -> Result<()> {
        let kind = &ch.resource.kind;
        let r = self.registry.get(kind).ok_or_else(|| Error::Invalid {
            message: format!("no Reversible registered for kind {kind:?}"),
        })?;
        r.apply_inverse(ch).await
    }

    async fn dispatch_forward(&self, ch: &Change) -> Result<()> {
        let kind = &ch.resource.kind;
        let r = self.registry.get(kind).ok_or_else(|| Error::Invalid {
            message: format!("no Reversible registered for kind {kind:?}"),
        })?;
        r.apply_forward(ch).await
    }
}

fn actor_filter(actor: &Actor) -> ChangeFilter {
    let mut f = ChangeFilter::default();
    match actor {
        Actor::User { subject } => {
            f.actor_kind = Some("user".into());
            f.actor_id = Some(subject.clone());
        }
        Actor::Agent { run_id, .. } => {
            f.actor_kind = Some("agent".into());
            f.actor_id = Some(run_id.clone());
        }
        Actor::System => {
            f.actor_kind = Some("system".into());
        }
    }
    // Look at the last ~64 actions; the next undo target is almost
    // always within the most recent page.
    f.limit = Some(64);
    f
}
