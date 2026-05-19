//! `require_role(Role)` — middleware factory rejecting requests
//! whose principal lacks the role.

use crate::role::Role;

/// Build a tower `Layer` that returns `403 Forbidden` for any
/// authenticated request whose principal's role is not `>= role`.
///
/// Anonymous requests get `401 Unauthorized` first.
pub fn require_role(_role: Role) {
    // TODO(ap): concrete `Layer` impl lands with the principal
    // extractor. Public surface locked.
}
