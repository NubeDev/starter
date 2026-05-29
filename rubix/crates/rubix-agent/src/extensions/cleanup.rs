//! Rubix-supplied [`CleanupProvider`]s — the *knowledge* half of the
//! extension data-cleanup split (scope §4).
//!
//! The reusable **mechanism** (discover → dry-run → audited, idempotent
//! purge) lives upstream in `starter-ext-server`; the built-in
//! enablement-row + UI/i18n-cache providers auto-register there. This
//! module supplies the two providers that need rubix-only knowledge:
//!
//! - [`WarehouseCleanupProvider`] — lists and drops the `com_<id>__*`
//!   warehouse tables (and any continuous aggregates) an extension owns,
//!   scoped *strictly* to the extension's sanitised namespace prefix so
//!   it can never touch another extension's data. Mirrors the DDL naming
//!   in [`crate::boot`]'s `extension_tables` /
//!   [`crate::extensions::warehouse_write`].
//! - [`SkillCleanupProvider`] — removes the live
//!   [`starter_skills::SkillRegistry`] entries a bundle contributed via
//!   `contributes.skills[]`, so an uninstalled extension's skills stop
//!   surfacing immediately (the in-memory counterpart to dropping the
//!   bundle directory).
//!
//! Both are registered on the `ExtensionAdminBuilder` at boot
//! ([`crate::boot::build_extension_admin`]); the upstream orchestrator
//! logs every destructive step with the caller principal.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_host::ExtensionRegistry;
use starter_ext_server::{CleanupError, CleanupItem, CleanupKind, CleanupProvider};
use starter_ext_spi::{ExtensionId, Manifest};
use starter_flow_spi::skill::SkillId;
use starter_skills::SkillRegistry;

use crate::extensions::warehouse_write::sanitize_extension_id;

/// The `<sanitize(id)>__` prefix every table/cagg an extension owns
/// shares. Strict scoping: a label that does not start with this prefix
/// is never dropped.
fn namespace_prefix(id: &ExtensionId) -> String {
    format!("{}__", sanitize_extension_id(id))
}

// ---------------------------------------------------------------------------
// Warehouse tables + continuous aggregates
// ---------------------------------------------------------------------------

/// Drops the `com_<id>__*` warehouse tables (and continuous aggregates)
/// an extension owns. Namespace-scoped: every candidate name is
/// re-checked against the extension's sanitised prefix before any DROP.
pub struct WarehouseCleanupProvider {
    pool: PgPool,
}

impl WarehouseCleanupProvider {
    /// Build against the warehouse [`PgPool`] — the same pool
    /// `create_extension_tables` issues its DDL through.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Tables in the current schema whose name starts with `prefix`.
    /// Filtering happens in Rust (not a SQL `LIKE`) because the prefix
    /// itself contains `_`, which `LIKE` treats as a wildcard — an
    /// exact `starts_with` keeps the scope strict.
    async fn matching_tables(&self, prefix: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT tablename FROM pg_tables WHERE schemaname = current_schema()")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|(name,)| name)
            .filter(|name| name.starts_with(prefix))
            .collect())
    }

    /// Continuous aggregates whose view name starts with `prefix`.
    /// Best-effort: when TimescaleDB is absent the
    /// `timescaledb_information` schema does not exist, so the query
    /// errors — we treat that as "no caggs" rather than failing cleanup.
    async fn matching_caggs(&self, prefix: &str) -> Vec<String> {
        let rows: Result<Vec<(String,)>, _> =
            sqlx::query_as("SELECT view_name FROM timescaledb_information.continuous_aggregates")
                .fetch_all(&self.pool)
                .await;
        match rows {
            Ok(rows) => rows
                .into_iter()
                .map(|(name,)| name)
                .filter(|name| name.starts_with(prefix))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Best-effort total relation size in bytes for a table.
    async fn table_bytes(&self, name: &str) -> Option<u64> {
        let size: Result<(i64,), _> =
            sqlx::query_as("SELECT pg_total_relation_size(format('%I', $1))")
                .bind(name)
                .fetch_one(&self.pool)
                .await;
        size.ok().and_then(|(b,)| u64::try_from(b).ok())
    }
}

#[async_trait]
impl CleanupProvider for WarehouseCleanupProvider {
    async fn discover(&self, id: &ExtensionId, _manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        let prefix = namespace_prefix(id);
        let mut items = Vec::new();
        // Continuous aggregates first so the dry-run lists them above
        // the base tables they read from.
        for view in self.matching_caggs(&prefix).await {
            items.push(CleanupItem {
                kind: CleanupKind::WarehouseTable,
                label: view,
                bytes: None,
            });
        }
        match self.matching_tables(&prefix).await {
            Ok(tables) => {
                for table in tables {
                    let bytes = self.table_bytes(&table).await;
                    items.push(CleanupItem {
                        kind: CleanupKind::WarehouseTable,
                        label: table,
                        bytes,
                    });
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "starter_ext_server::cleanup",
                    id = %id.as_str(),
                    err = %e,
                    "warehouse cleanup discover: listing tables failed",
                );
            }
        }
        items
    }

    async fn purge(&self, id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError> {
        let prefix = namespace_prefix(id);
        let mut errors: Vec<String> = Vec::new();
        for item in items
            .iter()
            .filter(|i| i.kind == CleanupKind::WarehouseTable)
        {
            let name = &item.label;
            // Strict namespace guard — never drop an object outside the
            // extension's own `com_<id>__*` namespace, even if a caller
            // hands us a hand-crafted item list.
            if !name.starts_with(&prefix) {
                tracing::warn!(
                    target: "starter_ext_server::cleanup",
                    id = %id.as_str(),
                    label = %name,
                    "refusing to drop object outside extension namespace",
                );
                continue;
            }
            // Resolve the object class so we issue the right DROP. A
            // `None` means it was already dropped (e.g. cascaded away by
            // a base-table drop) — idempotent no-op.
            let relkind: Option<(String,)> =
                sqlx::query_as("SELECT relkind::text FROM pg_class WHERE relname = $1")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap_or(None);
            let stmt = match relkind.as_ref().map(|(k,)| k.as_str()) {
                // Materialised view or a TimescaleDB continuous aggregate
                // (exposed as a plain view) — DROP MATERIALIZED VIEW.
                Some("m") | Some("v") => {
                    format!("DROP MATERIALIZED VIEW IF EXISTS \"{name}\" CASCADE")
                }
                // Ordinary / partitioned table — CASCADE also removes any
                // dependent continuous aggregates in one shot.
                Some(_) => format!("DROP TABLE IF EXISTS \"{name}\" CASCADE"),
                None => continue,
            };
            if let Err(e) = sqlx::query(&stmt).execute(&self.pool).await {
                errors.push(format!("{name}: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(CleanupError::new(errors.join("; ")))
        }
    }
}

// ---------------------------------------------------------------------------
// Contributed skills
// ---------------------------------------------------------------------------

/// Removes the live [`SkillRegistry`] entries a bundle contributed via
/// `contributes.skills[]`. Reads the contributed bundle directories off
/// disk to recover each `SKILL.md`'s id, then calls
/// [`SkillRegistry::remove`] for those present in the registry.
pub struct SkillCleanupProvider {
    skills: Arc<SkillRegistry>,
    extensions: Arc<ExtensionRegistry>,
}

impl SkillCleanupProvider {
    /// Build against the host's live skill registry and the sealed
    /// extension registry (used to resolve each bundle's on-disk dir).
    pub fn new(skills: Arc<SkillRegistry>, extensions: Arc<ExtensionRegistry>) -> Self {
        Self { skills, extensions }
    }

    /// Resolve the skill ids a bundle contributed, by walking each
    /// declared `contributes.skills[].dir` one level deep for `SKILL.md`
    /// bundles. Mirrors the upstream registry's one-level walk.
    fn contributed_skill_ids(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<SkillId> {
        let Some(record) = self.extensions.get_by_id_str(id.as_str()) else {
            return Vec::new();
        };
        let Some(manifest) = manifest.or(record.manifest.as_ref()) else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        for entry in &manifest.contributes.skills {
            let root = record.bundle_dir.join(&entry.dir);
            collect_skill_ids(&root, &mut ids);
        }
        ids
    }
}

/// Walk `root` one level deep, loading every `SKILL.md` bundle's id.
/// Errors (unreadable dir, malformed bundle) are skipped silently —
/// cleanup is best-effort.
fn collect_skill_ids(root: &Path, out: &mut Vec<SkillId>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let candidate = entry.path();
        if !candidate.join("SKILL.md").is_file() {
            continue;
        }
        if let Ok(bundle) = starter_skills::load_bundle(&candidate) {
            out.push(bundle.skill.id);
        }
    }
}

#[async_trait]
impl CleanupProvider for SkillCleanupProvider {
    async fn discover(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        self.contributed_skill_ids(id, manifest)
            .into_iter()
            .filter(|sid| self.skills.get(sid).is_some())
            .map(|sid| CleanupItem {
                kind: CleanupKind::Skill,
                label: sid.to_string(),
                bytes: None,
            })
            .collect()
    }

    async fn purge(&self, _id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError> {
        for item in items.iter().filter(|i| i.kind == CleanupKind::Skill) {
            match SkillId::new(&item.label) {
                Ok(sid) => {
                    self.skills.remove(&sid);
                }
                Err(_) => {
                    // A non-parseable label can never name a live skill;
                    // nothing to remove.
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_host::Loader;
    use starter_skills::{ContributedSkill, InMemoryApprovalStore};

    #[test]
    fn namespace_prefix_is_double_underscore_scoped() {
        let id = ExtensionId::new("com.acme.power").unwrap();
        assert_eq!(namespace_prefix(&id), "com_acme_power__");
    }

    #[tokio::test]
    async fn skill_provider_discovers_and_purges_contributed_skills() {
        // Lay down a bundle dir declaring a contributed skill, then load
        // it through the real two-phase loader so the record carries a
        // genuine manifest + bundle_dir.
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("com.acme.skills");
        let skill_dir = bundle.join("skills").join("greet");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nid: com.acme.skills.greet\ndescription: smoke\ntrust: quarantined\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            bundle.join("block.yaml"),
            r#"v: 1
id: com.acme.skills
version: 0.1.0
display_name: "Skill Test"
runtime: { kind: process, bin: ./bin/skill-test }
contributes:
  skills:
    - dir: skills
"#,
        )
        .unwrap();

        let mut extensions = ExtensionRegistry::new();
        let records = Loader::scan(dir.path()).validate_all();
        let _ = Loader::commit(records, &mut extensions);
        extensions.seal();
        let extensions = Arc::new(extensions);

        let skills = Arc::new(
            SkillRegistry::builder()
                .with_approval_store(InMemoryApprovalStore::new())
                .extend(vec![ContributedSkill::new(skill_dir)])
                .build()
                .await
                .unwrap(),
        );

        let id = ExtensionId::new("com.acme.skills").unwrap();
        let provider = SkillCleanupProvider::new(skills.clone(), extensions);

        // discover sees the live contributed skill.
        let items = provider.discover(&id, None).await;
        assert_eq!(items.len(), 1, "one contributed skill");
        assert_eq!(items[0].kind, CleanupKind::Skill);
        assert_eq!(items[0].label, "com.acme.skills.greet");

        // purge removes it from the live registry; re-discover is empty.
        provider.purge(&id, &items).await.unwrap();
        assert!(
            skills
                .get(&SkillId::new("com.acme.skills.greet").unwrap())
                .is_none(),
            "skill removed from registry"
        );
        assert!(provider.discover(&id, None).await.is_empty());

        // Idempotent: purging again is a no-op.
        provider.purge(&id, &items).await.unwrap();
    }
}
