//! `GarageBlobStore` — `BlobStore` impl that delegates to
//! [`starter_blob_s3::S3BlobStore`] over Garage's S3-compatible
//! endpoint and ties the lifecycle to a [`crate::GarageAdmin`].
//!
//! # Why a separate wrapper rather than a `type alias`
//!
//! An alias would let consumers reach Garage by typing
//! `S3BlobStore::open(...)` directly. That is technically the same
//! data plane but skips the Garage-specific construction sequence
//! (admin probe → ensure bucket → mint key → wire credentials). The
//! wrapper keeps the right construction path on the type the
//! consumer reaches for, and the [`BlobStore`] trait surface is
//! `delegate-impl` so no domain leakage sneaks in (B1 still holds
//! by construction).

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use starter_blob_s3::{S3BlobStore, S3BlobStoreConfig, S3BlobStoreError, S3Credentials};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobStore, ListPage, PresignOp,
    PresignedUrl, PutOptions,
};

use crate::admin::{ClusterStatus, GarageAdmin, GarageAdminError, LayoutInfo};

/// Errors specific to constructing a [`GarageBlobStore`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GarageBlobStoreError {
    /// Failure setting up the S3 data plane.
    #[error("garage data-plane setup: {0}")]
    DataPlane(#[from] S3BlobStoreError),
    /// Failure on the admin API at construction time (health probe
    /// or layout introspection).
    #[error("garage admin probe: {0}")]
    Admin(#[from] GarageAdminError),
    /// Cluster reported unavailable at probe time; refuse to wire a
    /// store whose first write would fail anyway.
    #[error("garage cluster unavailable at startup")]
    ClusterUnavailable,
}

/// Garage-backed blob store. Implements [`BlobStore`] by delegating
/// every method to a wrapped [`S3BlobStore`].
#[derive(Clone)]
pub struct GarageBlobStore {
    inner: S3BlobStore,
    /// Stored snapshot from the startup layout probe. Operators read
    /// this via [`GarageBlobStore::layout`] for dashboards / logs.
    layout: LayoutInfo,
}

impl GarageBlobStore {
    /// Build with explicit Garage credentials (typically minted via
    /// [`GarageAdmin::create_key`]). The constructor probes
    /// `/v1/health` and `/v1/layout` so a misconfigured cluster
    /// fails at startup rather than on the first `put`.
    pub async fn open(
        s3: S3BlobStoreConfig,
        credentials: S3Credentials,
        admin: &GarageAdmin,
    ) -> Result<Self, GarageBlobStoreError> {
        let health = admin.health().await?;
        if matches!(health.status, ClusterStatus::Unavailable) {
            return Err(GarageBlobStoreError::ClusterUnavailable);
        }
        let layout = admin.layout().await?;
        let inner = S3BlobStore::open_with_credentials(s3, credentials).await?;
        Ok(Self { inner, layout })
    }

    /// Layout snapshot captured at startup. Stable for the lifetime
    /// of the store; consumers that need a live read call
    /// [`GarageAdmin::layout`] themselves.
    pub fn layout(&self) -> &LayoutInfo {
        &self.layout
    }

    /// Borrow the underlying S3 client for the rare consumer that
    /// needs Garage-specific request shapes the trait does not
    /// expose. Use sparingly — every reach for the raw client is a
    /// place B1 could erode.
    pub fn s3(&self) -> &S3BlobStore {
        &self.inner
    }
}

#[async_trait]
impl BlobStore for GarageBlobStore {
    fn backend_id(&self) -> &BackendId {
        self.inner.backend_id()
    }
    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        self.inner.put_bytes(key, bytes, opts).await
    }
    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        self.inner.put_stream(key, stream, opts).await
    }
    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        self.inner.get(blob_ref, range).await
    }
    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        self.inner.head(blob_ref).await
    }
    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        self.inner.delete(blob_ref).await
    }
    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        self.inner.list(prefix, cursor).await
    }
    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        self.inner.presign(blob_ref, op, ttl).await
    }
    async fn copy_server_side(
        &self,
        src: &BlobRef,
        dst_key: &BlobKey,
    ) -> Result<BlobRef, BlobError> {
        self.inner.copy_server_side(src, dst_key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GarageBlobStore` must remain `dyn BlobStore`-compatible so
    /// the SwapTest fixture can swap fs→garage with one line.
    #[allow(dead_code)]
    fn _object_safety(_: &dyn BlobStore) {}
}
