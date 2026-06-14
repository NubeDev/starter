//! Request/response of `POST /api/v1/audit/forget` — GDPR right-to-erasure.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Which user to erase from the audit ledger. Erasure tombstones the *content*
/// (`before`/`after`/`patch`) of every change the subject authored, within the
/// caller's tenant; the audit fact (who/when/what op) is preserved, as a
/// regulator still needs to see that an action occurred.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForgetRequest {
    /// The principal subject (stable user id) whose authored changes to scrub.
    pub subject: String,
}

/// Result of a forget request: how many ledger rows were tombstoned.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForgetResponse {
    /// Number of ledger rows whose payloads were nulled.
    pub tombstoned: u64,
}
