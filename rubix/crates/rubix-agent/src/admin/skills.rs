//! Project the bundled rubix skills into [`RegistryItem`]s.
//!
//! Today the rubix-agent does not thread a live
//! [`starter_skills::SkillRegistry`] into the admin state — the
//! skill substrate consumes the bundled `Dir` directly at boot.
//! This projection walks the same embedded directory and emits one
//! row per `<dir>/SKILL.md` entry; descriptions and allowed-tool
//! lists arrive when a live `SkillRegistry` handle is wired (a
//! follow-up to the admin surface).

use rubix_spi::dto::admin::{ItemSource, RegistryItem};
use serde_json::json;

/// Project every bundled `<goal>/SKILL.md` directory into an
/// admin row. The id is the directory name (e.g.
/// `flow-programmer`); the bundle path lives in
/// `metadata.bundle_dir`.
pub fn skill_items() -> Vec<RegistryItem> {
    rubix_skills::bundled()
        .dirs()
        .filter_map(|dir| {
            // Only directories carrying a SKILL.md are real skill
            // bundles; anything else is content (e.g. a
            // `resources/` shared folder).
            let has_skill = dir.files().any(|f| {
                f.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "SKILL.md")
                    .unwrap_or(false)
            });
            if !has_skill {
                return None;
            }
            let id = dir.path().file_name().and_then(|n| n.to_str())?.to_owned();
            let metadata = json!({
                "quarantined": false,
                "bundle_dir": dir.path().to_string_lossy(),
            });
            Some(
                RegistryItem::new(id.clone(), ItemSource::Builtin)
                    .with_label(id)
                    .with_metadata(metadata),
            )
        })
        .collect()
}
