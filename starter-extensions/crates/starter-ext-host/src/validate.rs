//! Semantic checks the loader runs after a manifest has parsed.
//!
//! Two checks live here in Phase 1:
//!
//! - **R4 namespace ownership.** Every id the extension contributes (a
//!   `contributes.tools[].id`, `contributes.cli[].id`, `contributes.rest[].id`,
//!   `contributes.grpc[].id`, `contributes.workers[].id`) must be the
//!   extension's own id or a dotted descendant. Reserved prefixes
//!   (`sys.*`, `starter.*`) cannot be claimed — `ExtensionId::new` already
//!   refuses them, so the manifest could not have parsed if the *extension's
//!   own id* was reserved.
//! - **R6 capability compatibility.** Every category named in `requires:`
//!   must appear in `capabilities:`. An empty allowlist (`http_out: []`) is
//!   a legal neutralised grant; *omitting* the category when `requires`
//!   names it is a hard load error.
//!
//! Both checks return [`starter_ext_spi::Error::Validation`] with a concrete
//! reason so the registry's failed-record `failure` field is human-readable.

use starter_ext_spi::id::RESERVED_PREFIXES;
use starter_ext_spi::{Capability, Error, Manifest, Result};

/// Run every semantic check against one parsed manifest. Returns the first
/// failure; per-extension isolation is the *caller's* responsibility
/// (`Loader::validate_all` calls this once per candidate and records each
/// failure independently).
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    check_namespace(m)?;
    check_capability_compatibility(m)?;
    Ok(())
}

fn check_namespace(m: &Manifest) -> Result<()> {
    let owner = &m.id;
    for t in &m.contributes.tools {
        if !owner.owns(&t.id) {
            return Err(Error::validation(format!(
                "contributes.tools[].id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                t.id,
                owner.as_str()
            )));
        }
    }
    for e in &m.contributes.cli {
        if !owner.owns(&e.id) {
            return Err(Error::validation(format!(
                "contributes.cli[].id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                e.id,
                owner.as_str()
            )));
        }
    }
    for e in &m.contributes.rest {
        if !owner.owns(&e.id) {
            return Err(Error::validation(format!(
                "contributes.rest[].id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                e.id,
                owner.as_str()
            )));
        }
    }
    for e in &m.contributes.grpc {
        if !owner.owns(&e.id) {
            return Err(Error::validation(format!(
                "contributes.grpc[].id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                e.id,
                owner.as_str()
            )));
        }
    }
    for e in &m.contributes.workers {
        if !owner.owns(&e.id) {
            return Err(Error::validation(format!(
                "contributes.workers[].id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                e.id,
                owner.as_str()
            )));
        }
    }
    // `contributes.nodes[].kind` per
    // `DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-3. Two
    // failure modes that must be reported distinctly so an operator
    // can act on the message:
    //
    //  1. Host-reserved prefix (`starter.*`, `sys.*`) — the kernel
    //     reserves these for built-in kinds; an extension can never
    //     claim one. This case is reported even when the
    //     extension's own id is namespace-disjoint from the kind
    //     (e.g. `com.nube.foo` trying to claim `starter.flow.mqtt`).
    //  2. Namespace mismatch — the kind does not start with a
    //     reserved prefix but still escapes the extension's
    //     subtree (`com.nube.foo` trying to claim `com.other.bar`).
    for e in &m.contributes.nodes {
        if let Some(prefix) = first_reserved_prefix(&e.kind) {
            return Err(Error::validation(format!(
                "contributes.nodes[].kind {:?} begins with host-reserved prefix {:?} \
                 (FLOW-NODES.md R-flow-node-3)",
                e.kind, prefix
            )));
        }
        if !owner.owns(&e.kind) {
            return Err(Error::validation(format!(
                "contributes.nodes[].kind {:?} escapes the extension's namespace {:?} \
                 (FLOW-NODES.md R-flow-node-3)",
                e.kind,
                owner.as_str()
            )));
        }
    }
    // `contributes.warehouse_templates[].name` per
    // `rubix/docs/scope/extensions-north-star` row 3. Same shape as
    // `contributes.nodes[].kind`: reserved prefixes are reported
    // distinctly from namespace-escape failures so an operator can
    // act on the message.
    for e in &m.contributes.warehouse_templates {
        if let Some(prefix) = first_reserved_prefix(&e.name) {
            return Err(Error::validation(format!(
                "contributes.warehouse_templates[].name {:?} begins with host-reserved \
                 prefix {:?} (rubix/docs/scope/extensions-north-star row 3)",
                e.name, prefix
            )));
        }
        // `warehouse.*` is reserved for the host's future promoted-builtin
        // tier (see rubix/docs/scope/extensions/extension-data-to-dashboard.md
        // §"Growing the common toolkit"). Treated identically to `starter.*` /
        // `sys.*`: hard reject so an extension cannot squat the namespace.
        // Kept inline here rather than added to `RESERVED_PREFIXES` to avoid
        // also rejecting extension *ids* like `warehouse.acme.*`, which the
        // SCOPE.md R4 layer already enforces.
        if e.name.split('.').next() == Some("warehouse") {
            return Err(Error::validation(format!(
                "contributes.warehouse_templates[].name {:?} begins with host-reserved \
                 prefix \"warehouse\" (rubix/docs/scope/extensions/extension-data-to-dashboard.md)",
                e.name
            )));
        }
        if !owner.owns(&e.name) {
            return Err(Error::validation(format!(
                "contributes.warehouse_templates[].name {:?} escapes the extension's \
                 namespace {:?} (rubix/docs/scope/extensions-north-star row 3)",
                e.name,
                owner.as_str()
            )));
        }
    }
    // `contributes.anomaly_rules[].id` per
    // `rubix/docs/scope/extensions-north-star` row B5. Same shape
    // as `contributes.nodes[].kind` + an extra rejection of
    // `builtin.*` so a manifest cannot shadow the in-process
    // detectors the cleaner short-circuits.
    for e in &m.contributes.anomaly_rules {
        if e.id.split('.').next() == Some("builtin") {
            return Err(Error::validation(format!(
                "contributes.anomaly_rules[].id {:?} begins with rule-host-reserved \
                 prefix \"builtin\" (rubix/docs/scope/extensions-north-star row B5)",
                e.id
            )));
        }
        if let Some(prefix) = first_reserved_prefix(&e.id) {
            return Err(Error::validation(format!(
                "contributes.anomaly_rules[].id {:?} begins with host-reserved \
                 prefix {:?} (rubix/docs/scope/extensions-north-star row B5)",
                e.id, prefix
            )));
        }
        if !owner.owns(&e.id) {
            return Err(Error::validation(format!(
                "contributes.anomaly_rules[].id {:?} escapes the extension's \
                 namespace {:?} (rubix/docs/scope/extensions-north-star row B5)",
                e.id,
                owner.as_str()
            )));
        }
    }
    Ok(())
}

/// Return the first host-reserved prefix `kind` matches against, if
/// any. Matching is on the first dot-segment so `starter.flow.mqtt`
/// matches `starter` but `starterly.weather` does not.
fn first_reserved_prefix(kind: &str) -> Option<&'static str> {
    let first_segment = kind.split('.').next()?;
    RESERVED_PREFIXES
        .iter()
        .find(|&&prefix| first_segment == prefix)
        .copied()
}

fn check_capability_compatibility(m: &Manifest) -> Result<()> {
    // `requires:` entries whose id starts with `cap.` (e.g. `cap.http_out`)
    // name a capability *category* the extension needs at runtime. Entries
    // outside that prefix (e.g. `starter.spi.tool`) name a host *interface*
    // — not a capability — and are not part of the R6 compatibility check.
    //
    // Capability-category names line up with the YAML tag on the `Capability`
    // enum (`secrets`, `http_out`, `fs`, `wall_clock`, plus `custom:<name>`).
    for req in &m.requires {
        let Some(category) = req.id.strip_prefix("cap.") else {
            continue;
        };
        let granted = m
            .capabilities
            .iter()
            .any(|c| capability_matches(c, category));
        if !granted {
            return Err(Error::validation(format!(
                "extension requires capability {:?} but the manifest's `capabilities:` block \
                 does not include that category (SCOPE R6: omission is a load error; \
                 setting an empty allowlist is the way to neutralise a grant)",
                category
            )));
        }
    }
    Ok(())
}

fn capability_matches(c: &Capability, category: &str) -> bool {
    match (c, category) {
        (Capability::Secrets { .. }, "secrets") => true,
        (Capability::HttpOut { .. }, "http_out") => true,
        (Capability::Fs { .. }, "fs") => true,
        (Capability::WallClock { .. }, "wall_clock") => true,
        (Capability::WarehouseRead { .. }, "warehouse_read") => true,
        (Capability::EventBus { .. }, "event_bus") => true,
        (Capability::Ingest { .. }, "ingest") => true,
        (Capability::DashboardRead { .. }, "dashboard_read") => true,
        (Capability::DashboardWrite { .. }, "dashboard_write") => true,
        (Capability::AuthzCheck { .. }, "authz_check") => true,
        (Capability::Custom { name, .. }, c) => {
            // `custom:<name>` in `requires:` matches `Capability::Custom { name }`.
            c.strip_prefix("custom:")
                .map(|n| n == name)
                .unwrap_or(false)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::Manifest;

    fn parse(yaml: &str) -> Manifest {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn namespace_ok_for_dotted_descendants() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
contributes:
  tools:
    - id: com.acme.weather.current
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn namespace_rejects_sibling_id() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
contributes:
  tools:
    - id: com.other.thing.t
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn anomaly_rule_id_ok_for_dotted_descendant() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
contributes:
  anomaly_rules:
    - id: com.acme.weather.spike
      tool_id: com.acme.weather.spike_check
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn anomaly_rule_id_rejects_sibling_namespace() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
contributes:
  anomaly_rules:
    - id: com.other.thing.spike
      tool_id: com.other.thing.spike_check
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn anomaly_rule_id_rejects_builtin_prefix() {
        // A manifest must not be able to shadow `builtin.nan` / etc.
        let m = parse(
            r#"
v: 1
id: com.acme.x
version: 0.1.0
display_name: "X"
runtime: { kind: builtin, crate_name: x }
contributes:
  anomaly_rules:
    - id: builtin.nan
      tool_id: com.acme.x.tool
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn anomaly_rule_id_rejects_starter_prefix() {
        let m = parse(
            r#"
v: 1
id: com.acme.x
version: 0.1.0
display_name: "X"
runtime: { kind: builtin, crate_name: x }
contributes:
  anomaly_rules:
    - id: starter.foo.rule
      tool_id: com.acme.x.tool
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn capability_compatibility_missing_grant_fails() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
requires:
  - { id: cap.http_out, version: "^1" }
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn capability_compatibility_empty_allowlist_is_neutralised_grant() {
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
requires:
  - { id: cap.http_out, version: "^1" }
capabilities:
  - kind: http_out
    authorities: []
"#,
        );
        // R6: empty allowlist is the legal neutralised form; not a load error.
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn nodes_namespace_ok_for_dotted_descendants() {
        let m = parse(
            r#"
v: 1
id: com.nube.mqtt
version: 0.1.0
display_name: "MQTT"
runtime: { kind: process, bin: ./bin/mqtt-driver }
contributes:
  nodes:
    - kind: com.nube.mqtt.publish
      settings_schema: schemas/publish.json
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn nodes_namespace_rejects_reserved_prefix() {
        // R-flow-node-3: a fixture extension contributing
        // `starter.flow.mqtt` is rejected with the reserved-prefix
        // error path (distinct from the namespace-mismatch path).
        let m = parse(
            r#"
v: 1
id: com.nube.foo
version: 0.1.0
display_name: "F"
runtime: { kind: process, bin: ./bin/x }
contributes:
  nodes:
    - kind: starter.flow.mqtt
      settings_schema: a.json
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        let Error::Validation(msg) = err else {
            panic!("expected Validation error");
        };
        assert!(
            msg.contains("host-reserved prefix") && msg.contains("starter"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn nodes_namespace_rejects_non_descendant() {
        // R-flow-node-3: a fixture extension contributing
        // `com.other.mqtt` under an extension id of `com.nube.foo`
        // is rejected with the namespace-mismatch error path.
        let m = parse(
            r#"
v: 1
id: com.nube.foo
version: 0.1.0
display_name: "F"
runtime: { kind: process, bin: ./bin/x }
contributes:
  nodes:
    - kind: com.other.mqtt
      settings_schema: a.json
"#,
        );
        let err = validate_manifest(&m).unwrap_err();
        let Error::Validation(msg) = err else {
            panic!("expected Validation error");
        };
        assert!(
            msg.contains("escapes the extension's namespace"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn requires_outside_cap_prefix_is_ignored_by_this_check() {
        // Interface dependencies (`starter.spi.tool`, …) are validated by
        // the host's interface registry, not by this capability check.
        let m = parse(
            r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "W"
runtime: { kind: builtin, crate_name: weather }
requires:
  - { id: starter.spi.tool, version: "^1" }
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn warehouse_templates_namespace_ok() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
contributes:
  warehouse_templates:
    - name: com.acme.charts.daily
      params_schema: schemas/daily.json
      tables: [samples]
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn warehouse_templates_rejects_reserved_prefix() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
contributes:
  warehouse_templates:
    - name: starter.flow.q
      params_schema: x.json
      tables: []
"#,
        );
        let Error::Validation(msg) = validate_manifest(&m).unwrap_err() else {
            panic!("expected Validation error");
        };
        assert!(
            msg.contains("host-reserved prefix") && msg.contains("starter"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn warehouse_templates_rejects_warehouse_prefix() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
contributes:
  warehouse_templates:
    - name: warehouse.bucketed
      params_schema: x.json
      tables: []
"#,
        );
        let Error::Validation(msg) = validate_manifest(&m).unwrap_err() else {
            panic!("expected Validation error");
        };
        assert!(
            msg.contains("host-reserved") && msg.contains("warehouse"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn warehouse_templates_rejects_non_descendant() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
contributes:
  warehouse_templates:
    - name: com.other.thing.q
      params_schema: x.json
      tables: []
"#,
        );
        let Error::Validation(msg) = validate_manifest(&m).unwrap_err() else {
            panic!("expected Validation error");
        };
        assert!(msg.contains("escapes the extension's namespace"), "{msg}");
    }

    #[test]
    fn cap_warehouse_read_grant_satisfies_require() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
requires:
  - { id: cap.warehouse_read, version: "^1" }
capabilities:
  - kind: warehouse_read
    tables: [samples]
"#,
        );
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn cap_warehouse_read_missing_grant_fails() {
        let m = parse(
            r#"
v: 1
id: com.acme.charts
version: 0.1.0
display_name: "C"
runtime: { kind: builtin, crate_name: charts }
requires:
  - { id: cap.warehouse_read, version: "^1" }
"#,
        );
        assert!(matches!(
            validate_manifest(&m).unwrap_err(),
            Error::Validation(_)
        ));
    }
}
