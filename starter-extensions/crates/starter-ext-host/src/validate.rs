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
    Ok(())
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
        let granted = m.capabilities.iter().any(|c| capability_matches(c, category));
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
        (Capability::Custom { name, .. }, c) => {
            // `custom:<name>` in `requires:` matches `Capability::Custom { name }`.
            c.strip_prefix("custom:").map(|n| n == name).unwrap_or(false)
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
}
