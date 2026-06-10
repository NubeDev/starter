//! [`PostInstallHook`] — materialise a freshly-installed bundle's contributed
//! query-kinds into `nexus_extension_query_kinds` at install time.
//!
//! The kernel runs this right after a `POST /extensions/install` tarball is
//! extracted + validated, *before* the restart that makes the extension's code
//! live in the sealed registry. Persisting the contributed kinds here (rather
//! than only at the next boot's load) means the rows exist the moment install
//! returns, so the next boot — and the admin dry-run cleanup manifest — already
//! see them. The dispatcher's in-memory `extension_kinds` registry is still
//! boot-sealed, so the kinds become *resolvable* on restart (the install
//! response carries `pending_restart: true`); this hook makes them **durable**
//! immediately, closing the window the kernel's `PostInstallHook` exists for.
//!
//! Idempotent — install is re-runnable, and [`extension_query_kind::upsert`]
//! re-lands the same definition on a re-install. A hook failure is logged and
//! does not fail the install (the boot-time load remains the backstop), per the
//! kernel's `PostInstallHook` contract.

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_server::{CleanupError, PostInstallHook};
use starter_ext_spi::{ExtensionId, Manifest};

use crate::extensions::contribute::contributed_query_kinds;
use nexus_store::extension_query_kind;

/// Materialises contributed query-kinds on install. Holds the metadata pool and
/// the installs root so it can resolve the just-unpacked bundle's files.
pub struct QueryKindPostInstall {
    metadata: PgPool,
    installs_dir: std::path::PathBuf,
}

impl QueryKindPostInstall {
    /// `installs_dir` is the same writable root passed to
    /// `ExtensionAdminBuilder::with_installs_dir`; the install handler unpacks
    /// `<installs_dir>/<extension_id>/…`, which is where this hook reads the
    /// bundle-relative `sql_file` / `params_schema` from.
    pub fn new(metadata: PgPool, installs_dir: std::path::PathBuf) -> Self {
        Self {
            metadata,
            installs_dir,
        }
    }
}

#[async_trait]
impl PostInstallHook for QueryKindPostInstall {
    async fn run(&self, id: &ExtensionId, manifest: &Manifest) -> Result<String, CleanupError> {
        let bundle_dir = self.installs_dir.join(id.as_str());
        let kinds = contributed_query_kinds(id.as_str(), &bundle_dir, manifest)
            .map_err(|e| CleanupError::new(e.to_string()))?;

        if kinds.is_empty() {
            return Ok("no query-kinds contributed".to_string());
        }

        let mut persisted = 0usize;
        for new in &kinds {
            extension_query_kind::upsert(&self.metadata, id.as_str(), new)
                .await
                .map_err(|e| {
                    CleanupError::new(format!("persist query-kind {}: {e}", new.name))
                })?;
            persisted += 1;
        }
        Ok(format!(
            "materialised {persisted} query-kind(s); live after restart"
        ))
    }
}
