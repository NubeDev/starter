//! `starter-blobs-example` — axum upload → SQLite-`BlobRef` →
//! presigned-GET round-trip end-to-end.
//!
//! Stage 5 of the blob-storage SCOPE. See `README.md` for the
//! wire shape. The wiring deliberately speaks to the
//! [`BlobStore`](starter_spi::blob::BlobStore) trait via
//! `Arc<dyn BlobStore>` so a deployment swaps engines without
//! touching the routes.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use starter_blob_memory::{
    router::router as memory_router, MemoryBlobStore, MemoryBlobStoreConfig,
};
use starter_spi::blob::BackendId;
use starter_store_sqlite::pool;

mod server;

const DEFAULT_BIND: &str = "127.0.0.1:8090";

#[tokio::main]
async fn main() -> Result<()> {
    let bind: SocketAddr = std::env::var("STARTER_BIND_ADDR")
        .unwrap_or_else(|_| DEFAULT_BIND.to_string())
        .parse()
        .context("parse STARTER_BIND_ADDR")?;

    // SQLite is in-memory: the example is self-contained, but the
    // `attachments` table shape (id, name, blob_ref_json) is the
    // load-bearing fact — a production deployment migrates the
    // same shape against a file-backed pool.
    let db = pool::connect("sqlite::memory:")
        .await
        .context("open in-memory sqlite")?;
    server::migrate(&db).await.context("apply schema")?;

    // The engine is mounted at /blobs, so the presigned URLs the
    // engine mints point back at this very process. Swap to
    // `starter-blob-fs` / `starter-blob-garage` to point at a
    // different host.
    let blob_base = format!("http://{bind}/blobs");
    let engine = MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new("memory:blobs-example"),
        public_base_url: blob_base,
    });

    let app = server::router(db, Arc::new(engine.clone()))
        .merge(axum::Router::new().nest("/blobs", memory_router(engine)));

    eprintln!("starter-blobs-example listening on {bind}");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve")?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
