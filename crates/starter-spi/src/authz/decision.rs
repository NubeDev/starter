//! Value types describing an authorization decision and the
//! resource it was made against. Engines consume a [`ResourceRef`]
//! and emit a [`Decision`].

use serde::{Deserialize, Serialize};

/// Reference to the object an authorization check is being made
/// against. `id == None` is a collection-level / route-level check
/// ("may this user list flows at all?"); `id == Some(_)` is a
/// row-level check ("may this user update flow 42?").
///
/// `owner` is populated by the handler for row-level checks where
/// ownership matters — the engine can match
/// `principal.subject == object.owner` without a DB round-trip from
/// inside the engine. See SCOPE.md R5.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceRef {
    /// Resource kind. Must be registered in the
    /// [`super::ResourceRegistry`] (e.g. `"flows"`, `"users"`,
    /// `"secrets"`).
    pub kind: String,
    /// Resource id for row-level checks. `None` for collection /
    /// route-level checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Subject id of the resource owner, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

impl ResourceRef {
    /// Collection-level reference (no row id, no owner). Convenient
    /// for middleware that gates a whole route.
    pub fn collection(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: None,
            owner: None,
        }
    }

    /// Row-level reference with id but no owner.
    pub fn row(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: Some(id.into()),
            owner: None,
        }
    }

    /// Set the owning subject for ownership rules.
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }
}

/// Outcome of a [`super::PolicyEngine::check`] call.
///
/// Denials carry a stable `reason` code (SCOPE.md R9) so the HTTP
/// layer can map them to `403 { "error": "<reason>" }` without
/// leaking rule details to callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum Decision {
    /// Permission granted. The optional `matched_rule` id lets
    /// audit logs and the `/v1/authz/check` dry-run endpoint
    /// explain *which* rule allowed the action.
    Allow {
        /// Identifier of the rule that produced the allow, if the
        /// engine tracks rule ids (e.g. `StaticRbacEngine`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        matched_rule: Option<String>,
    },
    /// Permission refused. `reason` is a stable code; engines
    /// SHOULD use the documented set:
    /// `unknown_resource`, `no_matching_rule`, `explicit_deny`,
    /// `not_owner`, `role_missing`, `attribute_mismatch`.
    Deny {
        /// Stable machine-readable code surfaced as `error` in
        /// the HTTP 403 body.
        reason: String,
        /// Identifier of the rule that triggered the deny, if
        /// any. `None` for the catch-all `no_matching_rule` case.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        matched_rule: Option<String>,
    },
}

impl Decision {
    /// Convenience: `Allow` with no matched rule recorded.
    pub fn allow() -> Self {
        Self::Allow { matched_rule: None }
    }

    /// Convenience: `Allow` carrying the id of the rule that
    /// matched.
    pub fn allow_by(rule_id: impl Into<String>) -> Self {
        Self::Allow {
            matched_rule: Some(rule_id.into()),
        }
    }

    /// Convenience: `Deny` with a stable reason code.
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
            matched_rule: None,
        }
    }

    /// Convenience: `Deny` carrying the rule id that triggered it.
    pub fn deny_by(reason: impl Into<String>, rule_id: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
            matched_rule: Some(rule_id.into()),
        }
    }

    /// `true` if the decision allows the action.
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}
