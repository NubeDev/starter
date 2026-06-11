//! Authz integration (P3, DOCS §10).
//!
//! Registers the `setup.templates` + `setup.runs` resource kinds and
//! exposes the default rules. The **per-template team check is NOT a
//! generic authz condition** — the condition engine sees only
//! `object.{kind,id,owner,tenant}`, not `allowed_teams` — so it lives as
//! a setup-layer Rust check ([`team_check`]) the run handler runs after
//! the coarse generic gate (DOCS §10 two-layer model).

use starter_spi::authz::{Ownership, ResourceSpec};
use starter_spi::auth::Principal;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::Template;

/// Resource kind for the template catalog.
pub const KIND_TEMPLATES: &str = "setup.templates";
/// Resource kind for run executions.
pub const KIND_RUNS: &str = "setup.runs";

/// The `setup.templates` resource spec (DOCS §10). Authors own their
/// templates; tenant-scoped so the cross-tenant predicate applies.
pub fn templates_spec() -> ResourceSpec {
    ResourceSpec::from_static_tenant_scoped(
        KIND_TEMPLATES,
        &["read", "create", "update", "delete", "run"],
        Ownership::Subject,
        "Setup templates",
        "Parameterized automations.",
    )
}

/// The `setup.runs` resource spec (DOCS §10). Launchers own their runs.
pub fn runs_spec() -> ResourceSpec {
    ResourceSpec::from_static_tenant_scoped(
        KIND_RUNS,
        &["read", "create", "cancel", "resume"],
        Ownership::Subject,
        "Setup runs",
        "Automation executions.",
    )
}

/// Register both setup resource specs into a registry that exposes
/// `register_spec` (e.g. `starter_authz::registry::StaticRegistry`). Call
/// at boot before the engine evaluates rules.
pub fn register_specs<R: RegisterSpec>(registry: &R) {
    registry.register_spec(templates_spec());
    registry.register_spec(runs_spec());
}

/// Minimal trait abstracting the concrete authz registry's
/// `register_spec`, so this crate need not depend on `starter-authz`'s
/// concrete `StaticRegistry` type at the SPI layer.
pub trait RegisterSpec {
    /// Register one resource spec.
    fn register_spec(&self, spec: ResourceSpec);
}

/// Bridge to the concrete `starter_authz::StaticRegistry` (available with
/// the `rest` feature, which pulls `starter-authz`). Lets a host call
/// `register_specs(&static_registry)` directly at boot.
#[cfg(feature = "rest")]
impl RegisterSpec for starter_authz::registry::StaticRegistry {
    fn register_spec(&self, spec: ResourceSpec) {
        starter_authz::registry::StaticRegistry::register_spec(self, spec);
    }
}

/// The default authz rules for the setup surface, as a TOML fragment
/// (DOCS §10). Compose into the deployment's `AuthzConfig`.
///
/// - writers manage their own templates (`condition = "owner"`),
/// - the coarse `run` gate is role/tenant only (the team check is the
///   setup-layer step), and
/// - launchers read/resume/cancel their own runs.
pub const DEFAULT_RULES_TOML: &str = r#"
# Authors (writers) manage their own templates; admins manage all.
[[rules]]
role = "writer"
resource = "setup.templates"
actions = ["read", "create", "update", "delete"]
condition = "owner"
effect = "allow"

# Coarse gate: who may attempt to RUN a template at all (role/tenant only).
# The data-dependent team check is the setup-layer Rust step, NOT a
# condition (DOCS §10).
[[rules]]
role = "writer"
resource = "setup.templates"
actions = ["run"]
effect = "allow"

# Launchers see and resume/cancel their own runs; admins see all.
[[rules]]
role = "*"
resource = "setup.runs"
actions = ["read", "resume", "cancel"]
condition = "owner"
effect = "allow"

[[rules]]
role = "*"
resource = "setup.runs"
actions = ["create"]
effect = "allow"
"#;

/// The setup-layer team check (DOCS §10 step 2). After the generic authz
/// `run` gate passes, assert the principal shares a team with the
/// template's `allowed_teams` (empty = any team in tenant). This is the
/// data-dependent predicate the condition engine cannot express, because
/// it never sees the object's `allowed_teams`.
///
/// Also enforces tenant isolation defence-in-depth: a template bound to a
/// tenant may only be run by a principal in that tenant (the generic
/// `tenant_scoped` predicate is the primary guard; this is a backstop).
pub fn team_check(template: &Template, principal: &Principal) -> SetupResult<()> {
    // Tenant backstop: a tenant-bound template (non-global) requires a
    // matching principal tenant.
    if let Some(t) = &template.access.tenant_id {
        if t != starter_setup_spi::store::GLOBAL_TENANT_SENTINEL
            && principal.tenant_id.as_deref() != Some(t.as_str())
            && !principal.is_super_admin()
        {
            return Err(SetupError::Forbidden(format!(
                "template belongs to tenant '{t}', principal is not in it"
            )));
        }
    }
    if template.access.team_allows(&principal.teams) {
        Ok(())
    } else {
        Err(SetupError::Forbidden(
            "principal shares no team with the template's allowed_teams".into(),
        ))
    }
}
