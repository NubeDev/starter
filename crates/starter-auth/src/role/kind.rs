//! The three built-in roles.

use serde::{Deserialize, Serialize};

/// Coarse permission level. Stored as a string in the users table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only access. Cannot mutate any resource.
    Reader,
    /// Can create, update, and delete the resources they own.
    Writer,
    /// Full access including user management.
    Admin,
}
