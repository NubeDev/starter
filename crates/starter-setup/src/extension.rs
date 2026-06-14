//! Extension seam (P5, DOCS §9 "Bundled templates").
//!
//! On enable, the host reads each `contributes.setup_templates[]` entry from an
//! extension's `block.yaml`, loads the referenced YAML envelope from the bundle
//! directory, runs it through the **same import path** REST
//! `/setup/templates/import` uses (envelope → nested `flow` → `FlowBody` →
//! node-kind validation), and `TemplateStore::put`s it with
//! `source = Extension { ext_id }`. Disabling the extension removes its
//! templates (by `(id, version)` under the global tenant).
//!
//! This module is host-agnostic: it takes a bundle directory + the parsed
//! contribution entries, so it works with any extension host that surfaces
//! those (nexus, rubix, the starter host). It is gated behind the
//! `extensions` feature so the core run service stays dep-light.

use std::path::Path;
use std::sync::Arc;

use starter_flow::registry::NodeKindRegistry;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::{TemplateId, TemplateSource};
use starter_setup_spi::store::TemplateStore;

use crate::import::import_template_yaml;

/// One bundled-template contribution: its declared id and the bundle-relative
/// path to the YAML envelope. Mirrors
/// `starter_ext_spi::manifest::ContributeSetupTemplate` without depending on it,
/// so this crate composes with any host's manifest type.
#[derive(Debug, Clone)]
pub struct SetupTemplateContribution {
    /// Declared template id (should match the envelope's `id:`).
    pub id: String,
    /// Bundle-relative path to the envelope file.
    pub file: String,
}

/// Import every bundled template from an extension into `store`, validating
/// each against `kinds` and recording `source = Extension { ext_id }`
/// (DOCS §9). Returns the imported template ids. The first failure aborts and
/// is returned (so a host can surface a bad bundle at enable time).
///
/// `bundle_dir` is the extension's bundle root; each contribution's `file` is
/// resolved relative to it. Templates are imported under the **global**
/// catalog (`tenant_id = None`) per §5 — all tenants inherit them and may
/// override with a same-`(id, version)` tenant row.
pub async fn import_bundled_templates<TS: TemplateStore>(
    bundle_dir: &Path,
    ext_id: &str,
    contributions: &[SetupTemplateContribution],
    store: &TS,
    kinds: &Arc<NodeKindRegistry>,
) -> SetupResult<Vec<TemplateId>> {
    let mut imported = Vec::new();
    for c in contributions {
        let path = bundle_dir.join(&c.file);
        let yaml = std::fs::read_to_string(&path).map_err(|e| {
            SetupError::InvalidYaml(format!("reading {}: {e}", path.display()))
        })?;
        let template = import_template_yaml(
            &yaml,
            None, // global catalog — DOCS §5
            TemplateSource::Extension {
                ext_id: ext_id.to_string(),
            },
            kinds,
        )
        .await?;
        // Defence: the declared id should match the envelope id.
        if template.id.0 != c.id {
            return Err(SetupError::InvalidBinding(format!(
                "bundled template id '{}' does not match envelope id '{}'",
                c.id, template.id.0
            )));
        }
        let id = store.put(template).await?;
        imported.push(id);
    }
    Ok(imported)
}

/// Remove every bundled template an extension contributed (DOCS §9
/// "Disabling the extension removes its templates"). Looks each up to learn its
/// version, then deletes the global-catalog row. Missing templates are skipped.
pub async fn remove_bundled_templates<TS: TemplateStore>(
    contributions: &[SetupTemplateContribution],
    store: &TS,
) -> SetupResult<()> {
    for c in contributions {
        let id = TemplateId(c.id.clone());
        // Find the global-catalog version(s) and delete. We look up "latest"
        // and delete that version; bundles ship one version per id.
        if let Some(t) = store.get(None, &id, None).await? {
            store.delete(None, &id, t.version).await?;
        }
    }
    Ok(())
}

/// Convenience: extract setup-template contributions from a host manifest's
/// already-parsed entries. The host calls this with whatever shape it parsed
/// `contributes.setup_templates[]` into — here expressed as `(id, file)` pairs
/// — keeping this crate free of a hard dependency on the extension SPI.
pub fn contributions_from_pairs<I, A, B>(pairs: I) -> Vec<SetupTemplateContribution>
where
    I: IntoIterator<Item = (A, B)>,
    A: Into<String>,
    B: Into<String>,
{
    pairs
        .into_iter()
        .map(|(id, file)| SetupTemplateContribution {
            id: id.into(),
            file: file.into(),
        })
        .collect()
}

/// Re-export of the [`Template`] type for hosts that want to inspect what was
/// imported without depending on `starter-setup-spi` directly.
pub use starter_setup_spi::model::Template as ImportedTemplate;
