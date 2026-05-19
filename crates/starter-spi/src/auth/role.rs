//! Built-in roles. Coarse permission level carried on every
//! `Principal`. Stored as a lowercase string when persisted.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Coarse permission level. Three roles are enough for the common
/// case; consumers needing more wire their own `Authenticator` and
/// translate to/from this set at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only access. Cannot mutate any resource.
    Reader,
    /// Can create, update, and delete the resources they own.
    Writer,
    /// Full access including user management.
    Admin,
}
