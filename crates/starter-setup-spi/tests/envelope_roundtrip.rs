//! P0 acceptance: YAML envelope round-trips, SemVer orders, the team
//! check matches DOCS §10, and reserved trusted-identity slots are
//! recognised (DOCS §6/§9/§10).

use starter_setup_spi::envelope::TemplateEnvelope;
use starter_setup_spi::model::{SemVer, TemplateAccess, TemplateSource};
use starter_setup_spi::reserved;

const SAMPLE: &str = r#"
id: com.acme.add-device
version: 1.2.0
display_name: Add a device
category: Provisioning
icon: scan
input_schema:
  type: object
  required: [barcode, location]
  properties:
    barcode:  { type: string, title: "Scan barcode" }
    location: { type: string, title: "Install location" }
input_bindings:
  - { field: barcode,  slot: com.acme.lookup-model.barcode }
  - { field: barcode,  slot: com.acme.create-device.barcode }
  - { field: location, slot: com.acme.create-device.location }
output_bindings:
  - { slot: com.acme.create-device.device_id, field: device_id }
access:
  allowed_teams: [hvac-ops]
  run_role: writer
flow:
  nodes:
    - { id: com.acme.lookup-model,    kind: starter.flow.http-out }
    - { id: com.acme.create-device,   kind: com.acme.device.create }
    - { id: com.acme.register-sensor, kind: com.acme.sensor.register }
    - { id: com.acme.notify,          kind: starter.flow.tool-call }
  links:
    - { from: com.acme.lookup-model.out,    to: com.acme.create-device.in }
    - { from: com.acme.create-device.out,   to: com.acme.register-sensor.in }
    - { from: com.acme.register-sensor.out, to: com.acme.notify.in }
"#;

#[test]
fn envelope_parses_and_nests_flow_body() {
    let env = TemplateEnvelope::from_yaml(SAMPLE).expect("parse envelope");
    assert_eq!(env.id, "com.acme.add-device");
    assert_eq!(env.version, SemVer::new(1, 2, 0));
    assert_eq!(env.input_bindings.len(), 3);

    let body = env.flow_body().expect("nested flow body deserializes");
    // flow_id injected from the envelope id.
    assert_eq!(body.flow_id.as_str(), "com.acme.add-device");
    assert_eq!(body.nodes.len(), 4);
    assert_eq!(body.links.len(), 3);
}

#[test]
fn template_export_roundtrips_to_envelope() {
    let env = TemplateEnvelope::from_yaml(SAMPLE).expect("parse");
    let template = env
        .clone()
        .into_template(Some("acme".into()), TemplateSource::Api)
        .expect("into template");
    assert_eq!(template.access.tenant_id.as_deref(), Some("acme"));

    let yaml = template.to_envelope_yaml().expect("export yaml");
    let reparsed = TemplateEnvelope::from_yaml(&yaml).expect("reparse exported");
    // Structural round-trip: id/version/bindings/body survive.
    assert_eq!(reparsed.id, env.id);
    assert_eq!(reparsed.version, env.version);
    assert_eq!(reparsed.input_bindings, env.input_bindings);
    assert_eq!(
        reparsed.flow_body().unwrap().nodes.len(),
        env.flow_body().unwrap().nodes.len()
    );
}

#[test]
fn semver_orders_and_parses() {
    assert!(SemVer::new(1, 2, 0) > SemVer::new(1, 1, 9));
    assert!(SemVer::new(2, 0, 0) > SemVer::new(1, 9, 9));
    assert_eq!(SemVer::parse("0.1.0").unwrap(), SemVer::new(0, 1, 0));
    assert!(SemVer::parse("1.2").is_err());
    assert!(SemVer::parse("1.2.3.4").is_err());
    assert!(SemVer::parse("x.y.z").is_err());
}

#[test]
fn team_check_matches_docs_section_10() {
    // Empty allowed_teams = any team in tenant.
    let open = TemplateAccess::default();
    assert!(open.team_allows(&[]));
    assert!(open.team_allows(&["anything".into()]));

    let scoped = TemplateAccess {
        allowed_teams: vec!["hvac-ops".into()],
        ..Default::default()
    };
    assert!(scoped.team_allows(&["hvac-ops".into()]));
    assert!(scoped.team_allows(&["other".into(), "hvac-ops".into()]));
    assert!(!scoped.team_allows(&["other".into()]));
    assert!(!scoped.team_allows(&[]));
}

#[test]
fn reserved_slots_recognised() {
    assert!(reserved::is_reserved("caller_user_id"));
    assert!(reserved::is_reserved("caller_team_ids"));
    assert!(reserved::is_reserved("caller_tenant_id"));
    assert!(!reserved::is_reserved("barcode"));
}
