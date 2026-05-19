//! [`ThemeStore`] — the persistence seam every backend implements.

use async_trait::async_trait;

use crate::error::Result;

use super::{ThemeDocument, ThemeSaveInput};

/// Persistence operations the theme editor needs.
///
/// Object-safe: handlers hold an `Arc<dyn ThemeStore>` so the same
/// router compiles against sqlite, Postgres, or a consumer-written
/// backend without generic plumbing.
///
/// **Single-tenant contract.** Each method operates on the one
/// shared org-level theme. Multi-tenant deployments wrap the impl
/// (e.g. resolve the tenant from request state, then dispatch to a
/// per-tenant instance) — that concern is intentionally outside the
/// trait so simple consumers stay simple.
///
/// **`load` never errors on "not yet set".** Fresh deployments
/// return a blank [`ThemeDocument`] (empty token maps, default shell,
/// no asset URLs). The frontend layers its bundled defaults on top.
#[async_trait]
pub trait ThemeStore: Send + Sync {
    /// Read the current document. Returns
    /// [`ThemeDocument::default()`] when no row exists.
    async fn load(&self) -> Result<ThemeDocument>;

    /// Replace the styles + shell, returning the post-save document
    /// (asset URLs preserved from before the call).
    async fn save(&self, input: ThemeSaveInput) -> Result<ThemeDocument>;

    /// Persist the logo bytes. Returns the URL the GET endpoint
    /// will serve them at.
    async fn put_logo(&self, bytes: Vec<u8>, content_type: &str) -> Result<String>;

    /// Drop any stored logo. Idempotent — no-op when nothing is set.
    async fn delete_logo(&self) -> Result<()>;

    /// Persist the favicon bytes. Returns the URL the GET endpoint
    /// will serve them at.
    async fn put_favicon(&self, bytes: Vec<u8>, content_type: &str) -> Result<String>;

    /// Drop any stored favicon. Idempotent.
    async fn delete_favicon(&self) -> Result<()>;

    /// Read the logo bytes + content-type, if any.
    async fn read_logo(&self) -> Result<Option<(Vec<u8>, String)>>;

    /// Read the favicon bytes + content-type, if any.
    async fn read_favicon(&self) -> Result<Option<(Vec<u8>, String)>>;
}
