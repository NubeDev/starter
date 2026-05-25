//! Static-asset mount with SPA fallback.
//!
//! Wraps `tower_http::services::ServeDir` so any binary built on
//! `ServerBuilder` can host a built single-page-app bundle under an
//! arbitrary mount path. Requests that don't match a file inside the
//! `dist_dir` fall back to `index.html` so client-side routers
//! (TanStack Router, React Router, etc.) own the URL space below the
//! mount.
//!
//! The mount is opt-in. Call [`mount`] directly when composing a
//! router by hand, or
//! [`ServerBuilder::with_static_assets`](crate::ServerBuilder::with_static_assets)
//! from the fluent builder.

use std::path::PathBuf;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

/// Mount `dist_dir` at `mount_path` on `router`, falling back to
/// `index.html` for any path that does not resolve to a file on disk.
///
/// `mount_path` follows axum's `nest_service` rules — use `/foo` or
/// `/foo/bar`, not `/` (use a dedicated single-page mount path for SPA
/// hosting; the rest of the router keeps its own routes).
pub fn mount<S>(router: Router<S>, mount_path: &str, dist_dir: PathBuf) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let index = dist_dir.join("index.html");
    let serve_dir = ServeDir::new(&dist_dir).fallback(ServeFile::new(index));
    router.nest_service(mount_path, serve_dir)
}
