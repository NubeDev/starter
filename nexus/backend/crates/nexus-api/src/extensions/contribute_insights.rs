//! Read an extension's `contributes.insights[]` into nexus extension-insights.
//!
//! The dual of [`super::contribute`] for the post-query insight stage: an
//! extension's `contributes.insights[]` declare named Rhai transform scripts the
//! host materialises into the global `nexus_extension_insights` registry. This
//! module reads each entry's `script_file` (and optional `params_schema`) off
//! disk (`ExtensionRecord::bundle_dir`), enforces R4 namespace ownership, and
//! **compiles the script through the exact same sandbox a stored insight passes
//! at save time** ([`nexus_insights::compile_check`]) — so an extension can never
//! contribute a script that fails to compile. A script that does not compile, or
//! a name that escapes the extension's namespace, is rejected (the contribution
//! is refused), never silently materialised.
//!
//! The result feeds two callers: the boot loader (materialise every validated
//! extension's insights into `nexus_extension_insights`) and the install
//! post-hook (materialise a freshly-uploaded bundle's insights immediately).

use std::path::Path;

use starter_ext_spi::id::RESERVED_PREFIXES;
use starter_ext_spi::{ExtensionId, Manifest};

use nexus_store::extension_insight::NewExtensionInsight;

/// Why a contributed insight could not be turned into a registry row.
#[derive(Debug, thiserror::Error)]
pub enum ContributeInsightError {
    /// A bundle-relative file (`script_file` or `params_schema`) was missing or
    /// unreadable.
    #[error("extension `{extension}` insight `{insight}`: reading {path}: {source}")]
    File {
        extension: String,
        insight: String,
        path: String,
        source: std::io::Error,
    },
    /// The `params_schema` file held invalid JSON.
    #[error("extension `{extension}` insight `{insight}`: params_schema is not valid JSON: {source}")]
    Schema {
        extension: String,
        insight: String,
        source: serde_json::Error,
    },
    /// The insight name escaped the extension's reverse-DNS namespace (R4), or
    /// claimed a host-reserved prefix.
    #[error("extension `{extension}` insight `{insight}`: name escapes the extension namespace (SCOPE R4)")]
    Namespace { extension: String, insight: String },
    /// The script failed to compile under the insight sandbox. The same gate a
    /// tenant-saved insight passes (`POST /insights` validate).
    #[error("extension `{extension}` insight `{insight}`: script does not compile: {reason}")]
    Compile {
        extension: String,
        insight: String,
        reason: String,
    },
}

/// Read every insight an extension contributes via its `contributes.insights[]`,
/// compile-checking each. `bundle_dir` is the extension's on-disk root
/// (`ExtensionRecord::bundle_dir`); `extension_id` is the owner recorded on each
/// row.
///
/// Every entry must pass namespace ownership and compile clean, or the whole
/// call errors — a partial, half-validated contribution is never returned.
pub fn contributed_insights(
    extension_id: &str,
    bundle_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<NewExtensionInsight>, ContributeInsightError> {
    let owner = ExtensionId::new(extension_id).ok();
    let mut out = Vec::new();
    for entry in &manifest.contributes.insights {
        check_namespace(extension_id, owner.as_ref(), &entry.name)?;

        let script =
            read_bundle_file(extension_id, &entry.name, bundle_dir, &entry.script_file)?;

        // The keystone safety check: an extension-contributed script passes the
        // identical sandbox compile a tenant-saved insight does. A script that
        // does not compile is rejected here — the contribution is refused, never
        // persisted with a broken body.
        nexus_insights::compile_check(&script).map_err(|e| ContributeInsightError::Compile {
            extension: extension_id.to_string(),
            insight: entry.name.clone(),
            reason: e.to_string(),
        })?;

        let params_schema = match &entry.params_schema {
            Some(rel) => {
                let raw = read_bundle_file(extension_id, &entry.name, bundle_dir, rel)?;
                let parsed: serde_json::Value =
                    serde_json::from_str(&raw).map_err(|source| ContributeInsightError::Schema {
                        extension: extension_id.to_string(),
                        insight: entry.name.clone(),
                        source,
                    })?;
                Some(parsed)
            }
            None => None,
        };

        out.push(NewExtensionInsight {
            name: entry.name.clone(),
            script,
            params_schema,
        });
    }
    Ok(out)
}

/// Enforce R4 namespace ownership: an insight name must be the extension's own
/// id or a dotted descendant, and must not claim a host-reserved prefix. Mirrors
/// the host loader's check for `warehouse_templates[].name`; done here because
/// the insight contribution path is nexus-owned.
fn check_namespace(
    extension_id: &str,
    owner: Option<&ExtensionId>,
    name: &str,
) -> Result<(), ContributeInsightError> {
    let reserved = name
        .split('.')
        .next()
        .map(|seg| RESERVED_PREFIXES.contains(&seg))
        .unwrap_or(false);
    let owned = owner.map(|o| o.owns(name)).unwrap_or(false);
    if reserved || !owned {
        return Err(ContributeInsightError::Namespace {
            extension: extension_id.to_string(),
            insight: name.to_string(),
        });
    }
    Ok(())
}

/// Read a bundle-relative file, mapping IO errors to [`ContributeInsightError::File`]
/// with the insight context attached.
fn read_bundle_file(
    extension_id: &str,
    insight: &str,
    bundle_dir: &Path,
    rel: &str,
) -> Result<String, ContributeInsightError> {
    let path = bundle_dir.join(rel);
    std::fs::read_to_string(&path).map_err(|source| ContributeInsightError::File {
        extension: extension_id.to_string(),
        insight: insight.to_string(),
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_bundle() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extensions/com.nexus.hello")
    }

    /// The whole boot-side insight contribution path against the real shipped
    /// bundle: the manifest validates through the kernel loader, and its
    /// `contributes.insights[]` read + compile clean into registry rows.
    #[test]
    fn example_bundle_contributes_compile_clean_insights() {
        let mut registry = starter_ext_host::ExtensionRegistry::new();
        let records =
            starter_ext_host::Loader::scan(example_bundle().parent().unwrap()).validate_all();
        let outcome = starter_ext_host::Loader::commit(records, &mut registry);
        assert_eq!(outcome.failed, 0, "the shipped example bundle must validate");
        registry.seal();

        let record = registry
            .get_by_id_str("com.nexus.hello")
            .expect("com.nexus.hello is in the pack");
        let manifest = record.manifest.as_ref().expect("validated ⇒ manifest");

        let insights = contributed_insights("com.nexus.hello", &record.bundle_dir, manifest)
            .expect("shipped insight scripts must read + compile clean");
        assert!(
            insights.iter().any(|i| i.name == "com.nexus.hello.zscore"),
            "the demo zscore insight is contributed"
        );
    }

    /// A name outside the extension's namespace is refused.
    #[test]
    fn rejects_out_of_namespace_name() {
        let owner = ExtensionId::new("com.nexus.hello").ok();
        let err = check_namespace("com.nexus.hello", owner.as_ref(), "com.other.thing")
            .unwrap_err();
        assert!(matches!(err, ContributeInsightError::Namespace { .. }));
    }
}
