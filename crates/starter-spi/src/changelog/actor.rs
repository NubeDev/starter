//! Who caused a change.
//!
//! [`Actor::User::subject`] reuses [`crate::auth::Principal::subject`]
//! — there is no parallel `principal_id` field. See SCOPE §"The seam".

use serde::{Deserialize, Serialize};

/// Origin of a recorded change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    /// A signed-in user. `subject` matches `Principal::subject`.
    User {
        /// Subject id of the acting principal.
        subject: String,
    },
    /// An AI agent run. `run_id` is opaque to the changelog; consumers
    /// with `starter-ai` join it themselves.
    Agent {
        /// Identifier of the agent run.
        run_id: String,
        /// Model identifier (e.g. `"claude-..."`).
        model: String,
    },
    /// A starter-internal / background actor (cron, migration).
    System,
}
