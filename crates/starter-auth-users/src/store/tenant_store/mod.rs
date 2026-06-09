//! `TenantStore` — manages the Phase 7a tenants + memberships
//! tables. The reserved-slug list is enforced both at the DB
//! level (CHECK constraint in the migration) and here in the
//! application before INSERT — the DB is the last line of
//! defence; the application gives a friendly error.
//!
//! See `DOCS/auth/authz/SCOPE-EXT.md` R11/R12.
//!
//! Backend impls live in sibling files (one per backend) to keep
//! each file under R1 (≤ 400 lines).

use async_trait::async_trait;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteTenantStore;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PgTenantStore;

/// Maximum tenant tree depth (ADR-tenant-hierarchy). A create whose
/// resulting `depth` would exceed this is refused — a cheap runaway
/// backstop. Reseller chains are a handful of levels; 16 is far
/// above any real hierarchy.
pub const MAX_TENANT_DEPTH: i32 = 16;

/// Tenant row.
#[derive(Debug, Clone)]
pub struct TenantRecord {
    /// Stable id (UUID).
    pub id: String,
    /// URL-facing identifier.
    pub slug: String,
    /// Display name shown in UIs.
    pub display_name: String,
    /// Per-tenant override of the audit-log allow-sample rate.
    pub audit_allow_sample: Option<i32>,
    /// Parent tenant id (ADR-tenant-hierarchy). `None` is a root
    /// tenant. Immutable after create — re-parenting is unsupported.
    pub parent_id: Option<String>,
}

/// Team row (Phase 7b — R13).
#[derive(Debug, Clone)]
pub struct TeamRecord {
    /// Stable id (UUID).
    pub id: String,
    /// Tenant the team belongs to.
    pub tenant_id: String,
    /// Rule-stable slug. Immutable after create.
    pub slug: String,
    /// Display name shown in UIs. Mutable.
    pub display_name: String,
}

/// Membership row joining a user to a tenant with a role.
#[derive(Debug, Clone)]
pub struct MembershipRecord {
    /// Tenant the user belongs to.
    pub tenant_id: String,
    /// User id.
    pub user_id: String,
    /// Role within the tenant. One of `reader | writer | admin`.
    pub role: String,
    /// The user's email, populated when the read joins the users table. `None`
    /// for writes (add/patch return the membership without re-reading the user)
    /// and for stores that do not populate it — a human-readable label for
    /// member pickers.
    pub email: Option<String>,
}

/// Tenant-store failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TenantStoreError {
    /// Backing store failed.
    #[error("tenant store error: {0}")]
    Backend(String),
    /// Slug collided with another tenant or with the reserved list.
    #[error("tenant slug conflict: {0}")]
    SlugConflict(String),
    /// Slug is on the reserved list (rejected before the INSERT).
    #[error("tenant slug reserved: {0}")]
    ReservedSlug(String),
    /// Lookup found no row.
    #[error("tenant not found: {0}")]
    NotFound(String),
    /// `parent_id` referenced a tenant that does not exist
    /// (ADR-tenant-hierarchy).
    #[error("parent tenant not found: {0}")]
    ParentNotFound(String),
    /// Creating under this parent would exceed [`MAX_TENANT_DEPTH`].
    #[error("tenant tree too deep (max {MAX_TENANT_DEPTH}): {0}")]
    MaxDepthExceeded(String),
}

/// Reserved slugs — rejected by both the application and the
/// DB-level CHECK constraint. Adding a new entry here is one
/// migration + this list bump.
///
/// `system` was here historically but is now a real tenant row
/// seeded by migration `0007_system_tenant.sql`; the UNIQUE
/// constraint on `slug` prevents duplicates, so it does not need
/// to live in this list.
pub const RESERVED_SLUGS: &[&str] = &[
    "admin",
    "api",
    "auth",
    "v1",
    "v2",
    "static",
    "health",
    "metrics",
    "openapi",
    "extensions",
    "mcp",
    "tools",
    "default",
];

/// Returns true if `slug` is reserved (in the static list or
/// all-digits).
pub fn is_reserved_slug(slug: &str) -> bool {
    if RESERVED_SLUGS.iter().any(|r| *r == slug) {
        return true;
    }
    !slug.is_empty() && slug.bytes().all(|b| b.is_ascii_digit())
}

/// CRUD over tenants + memberships. The Phase 7a admin REST
/// routes (`/v1/tenants/*`) call into this trait.
#[async_trait]
pub trait TenantStore: Send + Sync {
    /// Insert a new tenant. Refuses reserved slugs with
    /// `ReservedSlug`; collides with `SlugConflict`.
    ///
    /// When `row.parent_id` is `Some`, the parent must exist
    /// (`ParentNotFound` otherwise) and the resulting depth must not
    /// exceed [`MAX_TENANT_DEPTH`] (`MaxDepthExceeded` otherwise).
    /// The closure rows (the new tenant's self row plus one row per
    /// inherited ancestor, each one deeper) are written in the same
    /// transaction as the tenant insert (ADR-tenant-hierarchy).
    async fn create_tenant(&self, row: &TenantRecord) -> Result<(), TenantStoreError>;

    /// List every tenant (used by super-admin views).
    async fn list_tenants(&self) -> Result<Vec<TenantRecord>, TenantStoreError>;

    /// Look up a tenant by id.
    async fn get_tenant(&self, id: &str) -> Result<Option<TenantRecord>, TenantStoreError>;

    /// Look up a tenant by slug.
    async fn get_tenant_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<TenantRecord>, TenantStoreError>;

    /// Patch display_name / audit_allow_sample. Slug is immutable
    /// per SCOPE-EXT.md (admins must re-create the tenant to
    /// rename the slug).
    async fn patch_tenant(
        &self,
        id: &str,
        display_name: Option<&str>,
        audit_allow_sample: Option<Option<i32>>,
    ) -> Result<(), TenantStoreError>;

    /// Add a membership.
    async fn add_member(&self, row: &MembershipRecord) -> Result<(), TenantStoreError>;

    /// Patch a membership's role.
    async fn patch_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), TenantStoreError>;

    /// Delete a membership.
    async fn remove_member(&self, tenant_id: &str, user_id: &str) -> Result<(), TenantStoreError>;

    /// List a user's memberships (used by login / OAuth callback
    /// to choose a tenant).
    async fn memberships_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError>;

    /// List the members of a tenant (used by admin UIs).
    async fn members_of_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError>;

    // ---------------------------------------------------------- teams (7b)

    /// Insert a new team. `SlugConflict` if `(tenant_id, slug)`
    /// already exists.
    async fn create_team(&self, row: &TeamRecord) -> Result<(), TenantStoreError>;

    /// Delete a team and (via FK CASCADE) its memberships.
    async fn delete_team(&self, team_id: &str) -> Result<(), TenantStoreError>;

    /// Look up a team by id.
    async fn get_team(&self, team_id: &str) -> Result<Option<TeamRecord>, TenantStoreError>;

    /// List the teams in a tenant.
    async fn list_teams(&self, tenant_id: &str) -> Result<Vec<TeamRecord>, TenantStoreError>;

    /// Add a user to a team.
    async fn add_team_member(&self, team_id: &str, user_id: &str) -> Result<(), TenantStoreError>;

    /// Remove a user from a team.
    async fn remove_team_member(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<(), TenantStoreError>;

    /// Return the team slugs a user belongs to within `tenant_id`.
    /// Used by the authenticator at session-mint / token-verify
    /// time to populate `Principal.teams` (R13). The list is a
    /// `Vec<String>` of **slugs**, not ids — rules reference teams
    /// by slug so that recreating a team with the same slug keeps
    /// the rule working.
    async fn team_slugs_for_user(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>, TenantStoreError>;

    // ------------------------------------------------- hierarchy (ADR)

    /// Return every tenant id in the subtree rooted at `tenant_id`,
    /// **inclusive** of `tenant_id` itself (the depth-0 self row).
    /// For a leaf tenant this is `[tenant_id]`; for a parent it also
    /// contains every descendant. Used at session-mint / token-verify
    /// to populate `Principal.tenant_scope` (ADR-tenant-hierarchy).
    /// An unknown `tenant_id` yields an empty vec (no closure rows).
    async fn subtree_ids(&self, tenant_id: &str) -> Result<Vec<String>, TenantStoreError>;

    /// True when `ancestor` is an ancestor of — or equal to —
    /// `descendant` (i.e. a closure row `(ancestor, descendant)`
    /// exists). Used to authorize provisioning: a caller may create a
    /// tenant under `parent` only when they administer `parent`.
    async fn is_ancestor(
        &self,
        ancestor: &str,
        descendant: &str,
    ) -> Result<bool, TenantStoreError>;

    /// List the direct children of `tenant_id` (closure depth = 1).
    /// Convenience for admin UIs rendering a tree.
    async fn list_children(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TenantRecord>, TenantStoreError>;

    /// List every tenant in the subtree rooted at `tenant_id`,
    /// inclusive, as full records (vs [`Self::subtree_ids`] which
    /// returns ids only). Convenience for admin UIs.
    async fn list_subtree(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TenantRecord>, TenantStoreError>;
}
