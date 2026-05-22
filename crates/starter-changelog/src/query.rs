//! Read-side query API over `starter_changes`.
//!
//! Backends implement [`ChangeLog`]; projection crates
//! (`starter-audit`, `starter-agent-log`) consume it through a
//! [`ChangelogVisibilityRegistry`] gate. No SQL lives here.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use starter_spi::changelog::{Actor, Change, ChangeId, GroupId};
use starter_spi::Result;

/// Filter for [`ChangeLog::list`]. Every field is optional; unset
/// fields mean "no constraint".
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ChangeFilter {
    /// Filter by actor kind (`"user"`, `"agent"`, `"system"`).
    pub actor_kind: Option<String>,
    /// Filter by actor id (principal subject or agent run id).
    pub actor_id: Option<String>,
    /// Filter agent actors by model.
    pub actor_model: Option<String>,
    /// Filter by `resource.kind`.
    pub resource_kind: Option<String>,
    /// Filter by `resource.id` (requires `resource_kind`).
    pub resource_id: Option<String>,
    /// Filter by group id (load a whole transaction).
    pub group_id: Option<GroupId>,
    /// Inclusive lower bound on `at`.
    pub since: Option<DateTime<Utc>>,
    /// Exclusive upper bound on `at`.
    pub until: Option<DateTime<Utc>>,
    /// Page size. Backends MUST cap this.
    pub limit: Option<u32>,
    /// Opaque cursor returned by a previous page.
    pub cursor: Option<String>,
}

/// One page of changes plus the cursor for the next page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePage {
    /// Rows in descending `at` order.
    pub items: Vec<Change>,
    /// Cursor to pass into the next [`ChangeFilter`]. `None` at end.
    pub next_cursor: Option<String>,
}

/// Read-side handle. Implemented by `starter-changelog-{sqlite,postgres}`.
#[async_trait]
pub trait ChangeLog: Send + Sync {
    /// Load one change by id.
    async fn get(&self, id: &ChangeId) -> Result<Option<Change>>;

    /// Load every change in a group, in ascending `at` order
    /// (undo walks this in reverse).
    async fn group(&self, id: &GroupId) -> Result<Vec<Change>>;

    /// Paged list under a filter.
    async fn list(&self, filter: &ChangeFilter) -> Result<ChangePage>;
}

/// Convenience: build a filter scoped to a single resource.
pub fn filter_for_resource(kind: impl Into<String>, id: impl Into<String>) -> ChangeFilter {
    ChangeFilter {
        resource_kind: Some(kind.into()),
        resource_id: Some(id.into()),
        ..Default::default()
    }
}

/// Convenience: build a filter scoped to a single actor.
pub fn filter_for_actor(actor: &Actor) -> ChangeFilter {
    match actor {
        Actor::User { subject } => ChangeFilter {
            actor_kind: Some("user".into()),
            actor_id: Some(subject.clone()),
            ..Default::default()
        },
        Actor::Agent { run_id, model } => ChangeFilter {
            actor_kind: Some("agent".into()),
            actor_id: Some(run_id.clone()),
            actor_model: Some(model.clone()),
            ..Default::default()
        },
        Actor::System => ChangeFilter {
            actor_kind: Some("system".into()),
            ..Default::default()
        },
    }
}
