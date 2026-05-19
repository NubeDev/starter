//! `require_scope(Scope)` — middleware factory rejecting requests
//! whose principal lacks the scope.

use crate::scope::Scope;

/// Build a tower `Layer` that returns `403 Forbidden` if the
/// authenticated principal does not carry `scope`.
pub fn require_scope(_scope: Scope) {
    // TODO(ap): concrete `Layer` impl lands with the principal
    // extractor.
}
