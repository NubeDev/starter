//! # starter-audit
//!
//! Read-only projection of the changelog filtered to `Actor::User`.
//! Every returned row is passed through the
//! [`ChangelogVisibilityRegistry`] gate so a confused-deputy read
//! cannot leak rows the principal would not otherwise see (SCOPE
//! §"Security & privacy").

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod routes;
pub use routes::audit_router;

use std::sync::Arc;

use starter_changelog::{ChangeFilter, ChangeLog, ChangePage, ChangelogVisibilityRegistry};
use starter_spi::auth::Principal;
use starter_spi::Result;

/// Visibility-gated user-audit projection.
pub struct AuditService {
    log: Arc<dyn ChangeLog>,
    visibility: Arc<ChangelogVisibilityRegistry>,
}

impl AuditService {
    /// Wrap a log + visibility registry.
    pub fn new(log: Arc<dyn ChangeLog>, visibility: Arc<ChangelogVisibilityRegistry>) -> Self {
        Self { log, visibility }
    }

    /// Paged list of user-authored changes. The caller's filter is
    /// merged with a forced `actor_kind = "user"` constraint; any
    /// `actor_kind` set by the caller is overwritten.
    pub async fn list(&self, principal: &Principal, mut filter: ChangeFilter) -> Result<ChangePage> {
        filter.actor_kind = Some("user".into());
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
