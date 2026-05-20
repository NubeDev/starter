//! Policy persistence. The trait is the seam; the sqlite +
//! postgres impls are feature-gated. `DbPolicyEngine`
//! (`engine::db`) wraps any [`PolicyStore`] with a cached
//! [`crate::StaticRbacEngine`].
//!
//! Schema lives in `migrations/starter_authz_sqlite/*` and
//! `migrations/starter_authz_postgres/*`. The wire shape mirrors
//! [`crate::config::AuthzConfig`] so a TOML policy can be imported
//! row-for-row.

use async_trait::async_trait;

use crate::config::{Assignment, Rule};

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::{SqlitePolicyStore, AUTHZ_SQLITE_MIGRATOR};

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::{PostgresPolicyStore, AUTHZ_POSTGRES_MIGRATOR};

/// Errors specific to policy persistence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PolicyStoreError {
    /// A row with the same primary key already exists.
    #[error("authz policy row already exists: {0}")]
    Conflict(String),
    /// Lookup found no matching row.
    #[error("authz policy row not found: {0}")]
    NotFound(String),
    /// Stored row failed to deserialize (corrupt `actions` JSON,
    /// invalid `effect`, etc.). Indicates schema drift or
    /// hand-editing of the table.
    #[error("authz policy row malformed: {0}")]
    Malformed(String),
    /// Backing store failed.
    #[error("authz policy store error: {0}")]
    Backend(String),
}

/// CRUD over assignments + rules. Admin REST routes call into
/// this trait; the [`super::engine::db::DbPolicyEngine`] reads
/// from it on cache reloads.
#[async_trait]
pub trait PolicyStore: Send + Sync + 'static {
    /// Snapshot every assignment, in stable order
    /// (`created_at ASC, id ASC`).
    async fn list_assignments(&self) -> Result<Vec<StoredAssignment>, PolicyStoreError>;

    /// Snapshot every rule, in evaluation order
    /// (`priority DESC, created_at ASC, id ASC`).
    async fn list_rules(&self) -> Result<Vec<StoredRule>, PolicyStoreError>;

    /// Insert a new assignment. `id` must be unique. Returns
    /// [`PolicyStoreError::Conflict`] when `(subject, role)`
    /// collides.
    async fn insert_assignment(&self, row: &StoredAssignment) -> Result<(), PolicyStoreError>;

    /// Delete an assignment by id. Idempotent — deleting a missing
    /// row returns [`PolicyStoreError::NotFound`] so handlers can
    /// surface `404`.
    async fn delete_assignment(&self, id: &str) -> Result<(), PolicyStoreError>;

    /// Insert a new rule. `id` must be unique.
    async fn insert_rule(&self, row: &StoredRule) -> Result<(), PolicyStoreError>;

    /// Update an existing rule. Returns [`PolicyStoreError::NotFound`]
    /// when `id` does not exist. The whole row is replaced — there
    /// is no partial-update story.
    async fn update_rule(&self, row: &StoredRule) -> Result<(), PolicyStoreError>;

    /// Delete a rule by id.
    async fn delete_rule(&self, id: &str) -> Result<(), PolicyStoreError>;
}

/// One row in `starter_authz_assignments`, with audit columns.
/// [`Assignment`] is the wire shape (no audit columns); this is
/// the on-disk shape.
#[derive(Debug, Clone)]
pub struct StoredAssignment {
    /// Primary key.
    pub id: String,
    /// Exact subject id or single-trailing-`*` glob.
    pub subject: String,
    /// Role name (matches `Rule::role` + the `Principal.role`
    /// lowercase variants).
    pub role: String,
    /// Subject id of the admin who created the row.
    pub created_by: String,
}

impl StoredAssignment {
    /// Drop the audit columns — used when feeding the row into
    /// [`crate::AuthzConfig`] for the in-memory engine cache.
    pub fn to_config(&self) -> Assignment {
        Assignment {
            subject: self.subject.clone(),
            roles: vec![self.role.clone()],
        }
    }
}

/// One row in `starter_authz_rules`, with audit columns. [`Rule`]
/// is the wire shape; this is the on-disk shape.
#[derive(Debug, Clone)]
pub struct StoredRule {
    /// Primary key.
    pub id: String,
    /// Role this rule applies to. `"*"` matches any authenticated
    /// principal.
    pub role: String,
    /// Resource kind. `"*"` matches any registered kind.
    pub resource: String,
    /// Actions. `["*"]` matches any action.
    pub actions: Vec<String>,
    /// Optional condition.
    pub condition: Option<String>,
    /// `"allow"` or `"deny"`.
    pub effect: String,
    /// Evaluation priority; higher wins on equal-priority allow
    /// ordering. Deny always wins on conflict regardless.
    pub priority: i32,
    /// Subject id of the admin who created the row.
    pub created_by: String,
}

impl StoredRule {
    /// Drop the audit columns — used when feeding the row into
    /// [`crate::AuthzConfig`] for the in-memory engine cache.
    /// Returns [`PolicyStoreError::Malformed`] when `effect` is
    /// not one of the documented values.
    pub fn to_config(&self) -> Result<Rule, PolicyStoreError> {
        let effect = match self.effect.as_str() {
            "allow" => crate::config::Effect::Allow,
            "deny" => crate::config::Effect::Deny,
            other => {
                return Err(PolicyStoreError::Malformed(format!(
                    "unknown effect `{other}` on rule {}",
                    self.id
                )))
            }
        };
        Ok(Rule {
            id: Some(self.id.clone()),
            role: self.role.clone(),
            resource: self.resource.clone(),
            actions: self.actions.clone(),
            condition: self.condition.clone(),
            effect,
            priority: self.priority,
        })
    }
}
