//! Policy file shape. TOML by convention; the same structure is
//! mirrored in the `starter_authz_assignments` and
//! `starter_authz_rules` tables of the future `DbPolicyEngine`
//! (Phase 3) so a consumer can start file-based and migrate to DB
//! without a semantic change.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Top-level config — what [`crate::StaticRbacEngine::from_config`]
/// consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzConfig {
    /// Whether to layer in the built-in
    /// `Reader` / `Writer` / `Admin` defaults from
    /// [`crate::defaults`] *before* user rules. SCOPE.md R7:
    /// zero-config upgrade from `require_role`.
    #[serde(default = "default_true")]
    pub default_policy: bool,

    /// Subject → role bindings. A single subject can hold multiple
    /// roles via multiple entries.
    #[serde(default)]
    pub assignments: Vec<Assignment>,

    /// Ordered rule list. Evaluated in declaration order; deny
    /// wins overall (SCOPE.md R3, deny-overrides).
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Default for AuthzConfig {
    fn default() -> Self {
        // Match the `#[serde(default = "default_true")]` so
        // `AuthzConfig::default()` and a TOML file with no
        // `default_policy` key behave the same — both load the
        // built-in defaults.
        Self {
            default_policy: true,
            assignments: Vec::new(),
            rules: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// One subject-to-role binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Assignment {
    /// Exact subject id, or a glob with a single trailing `*`
    /// (e.g. `"*@acme.com"`).
    pub subject: String,
    /// Roles granted to that subject. Free-form strings — they
    /// match `Rule::role` and the `Principal.role` enum's
    /// lowercase name (`"reader" | "writer" | "admin"`).
    pub roles: Vec<String>,
}

/// One policy rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    /// Stable identifier. Optional in TOML — synthesised at load
    /// time when absent so audit logs always have something to
    /// quote. The DB engine populates this from the primary key.
    #[serde(default)]
    pub id: Option<String>,

    /// Role this rule applies to. `"*"` matches any authenticated
    /// principal.
    pub role: String,

    /// Resource kind. `"*"` matches any registered kind. Unknown
    /// kinds (not in the registry) never reach rule evaluation —
    /// they short-circuit to `Decision::Deny { reason:
    /// "unknown_resource" }` per SCOPE.md R3.
    pub resource: String,

    /// Actions. `["*"]` matches any action on the resource.
    pub actions: Vec<String>,

    /// Optional condition. Either the magic keyword `"owner"`
    /// (matches `principal.subject == object.owner`) or an
    /// expression in the mini-language ([`crate::condition`]).
    #[serde(default)]
    pub condition: Option<String>,

    /// `Allow` or `Deny`. Deny wins on conflict (SCOPE.md R3).
    pub effect: Effect,

    /// Higher values are evaluated first. Currently informational —
    /// deny-overrides means priority cannot promote an allow over
    /// a matching deny — but the field is wire-stable for the
    /// future DB engine.
    #[serde(default)]
    pub priority: i32,

    /// Tenant scope of this rule (Phase 7a). `None` means a global
    /// rule — evaluated for every tenant. `Some(tenant_id)` means
    /// the rule is only considered when the principal is bound to
    /// that tenant. The super-admin sentinel `"*"` on
    /// `Principal.tenant_id` matches every tenant_id (used by
    /// cross-tenant admin tokens).
    #[serde(default)]
    pub tenant_id: Option<String>,

    /// Instance scope of this rule. `None` or `"*"` is kind-wide —
    /// the rule applies to every instance of `resource`. A concrete
    /// id narrows the rule to the single instance whose `object.id`
    /// equals it, which is how a per-resource grant (a grant on one
    /// immutable dashboard/page id) is expressed: the engine only
    /// matches the rule when the request targets that exact id.
    #[serde(default)]
    pub resource_id: Option<String>,
}

/// Allow or deny.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// Permit the (principal, action, resource) tuple.
    Allow,
    /// Refuse it. Wins on conflict.
    Deny,
}

impl AuthzConfig {
    /// Parse a TOML policy string.
    pub fn from_toml_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| Error::Config(e.to_string()))
    }

    /// Load from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let s = std::fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("read {}: {e}", path.as_ref().display())))?;
        Self::from_toml_str(&s)
    }
}
