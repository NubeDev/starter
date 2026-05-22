//! # starter-agent-log
//!
//! Read-only projection of the changelog filtered to `Actor::Agent`.
//! The join key is the opaque `Actor::Agent::run_id` — consumers with
//! `starter-ai` join it themselves (SCOPE §"Crates" — no
//! `starter-ai` dep here).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod routes;
pub use routes::agent_log_router;

use std::sync::Arc;

use starter_changelog::{ChangeFilter, ChangeLog, ChangePage, ChangelogVisibilityRegistry};
use starter_spi::auth::Principal;
use starter_spi::Result;

/// Visibility-gated agent-log projection.
pub struct AgentLogService {
    log: Arc<dyn ChangeLog>,
    visibility: Arc<ChangelogVisibilityRegistry>,
}

impl AgentLogService {
    /// Wrap a log + visibility registry.
    pub fn new(log: Arc<dyn ChangeLog>, visibility: Arc<ChangelogVisibilityRegistry>) -> Self {
        Self { log, visibility }
    }

    /// Paged list of agent-authored changes. Forces
    /// `actor_kind = "agent"`; the caller may further narrow by
    /// `actor_id` (run id) or `actor_model`.
    pub async fn list(&self, principal: &Principal, mut filter: ChangeFilter) -> Result<ChangePage> {
        filter.actor_kind = Some("agent".into());
        let page = self.log.list(&filter).await?;
        let items = page
            .items
            .into_iter()
            .filter(|ch| self.visibility.may_read(principal, ch))
            .collect();
        Ok(ChangePage {
            items,
            next_cursor: page.next_cursor,
        })
    }
}
