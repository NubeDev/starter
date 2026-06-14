//! P3 acceptance: the setup-layer team check (DOCS §10 step 2) — the
//! data-dependent predicate the authz condition engine cannot express —
//! plus the tenant backstop.

use starter_setup::authz::team_check;
use starter_setup_spi::envelope::TemplateEnvelope;
use starter_setup_spi::model::{TemplateAccess, TemplateSource};
use starter_spi::auth::{Principal, Role};

const TEMPLATE: &str = r#"
id: com.acme.add-device
version: 1.0.0
display_name: Add a device
input_schema: { type: object }
flow:
  nodes:
    - { id: com.acme.notify, kind: starter.flow.tool-call }
  links: []
"#;

fn template(tenant: Option<&str>, access: TemplateAccess) -> starter_setup_spi::model::Template {
    let mut t = TemplateEnvelope::from_yaml(TEMPLATE)
        .unwrap()
        .into_template(tenant.map(str::to_string), TemplateSource::Api)
        .unwrap();
    let mut access = access;
    access.tenant_id = tenant.map(str::to_string);
    t.access = access;
    t
}

fn principal(tenant: Option<&str>, teams: &[&str]) -> Principal {
    Principal {
        subject: "u-1".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: tenant.map(str::to_string),
        teams: teams.iter().map(|s| s.to_string()).collect(),
        tenant_scope: vec![],
        extra: serde_json::Value::Null,
    }
}

#[test]
fn empty_allowed_teams_lets_any_team_in_tenant_run() {
    let t = template(Some("acme"), TemplateAccess::default());
    // Same tenant, any team (even none).
    assert!(team_check(&t, &principal(Some("acme"), &[])).is_ok());
    assert!(team_check(&t, &principal(Some("acme"), &["whatever"])).is_ok());
}

#[test]
fn scoped_teams_require_membership() {
    let t = template(
        Some("acme"),
        TemplateAccess {
            allowed_teams: vec!["hvac-ops".into()],
            ..Default::default()
        },
    );
    assert!(team_check(&t, &principal(Some("acme"), &["hvac-ops"])).is_ok());
    assert!(team_check(&t, &principal(Some("acme"), &["other", "hvac-ops"])).is_ok());
    // Right tenant, wrong team → forbidden.
    assert!(team_check(&t, &principal(Some("acme"), &["other"])).is_err());
}

#[test]
fn tenant_backstop_blocks_cross_tenant_run() {
    let t = template(
        Some("acme"),
        TemplateAccess {
            allowed_teams: vec![],
            ..Default::default()
        },
    );
    // Different tenant → forbidden even with open teams.
    assert!(team_check(&t, &principal(Some("zzz"), &[])).is_err());
}

#[test]
fn super_admin_bypasses_tenant_backstop() {
    let t = template(Some("acme"), TemplateAccess::default());
    let mut p = principal(Some("*"), &[]);
    p.tenant_id = Some("*".into()); // super admin sentinel
    assert!(team_check(&t, &p).is_ok());
}
