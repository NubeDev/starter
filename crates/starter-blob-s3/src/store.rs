//! `S3BlobStore` — the production `BlobStore` for S3-compatible
//! endpoints.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use bytes::{Bytes, BytesMut};
use futures::stream::{BoxStream, StreamExt, TryStreamExt};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore, Etag,
    ListPage, PresignOp, PresignedUrl, PutOptions,
};
use tracing::debug;

use crate::error::map_sdk_error;
use crate::TRACE_TARGET;

/// Multipart-upload chunk size used by [`S3BlobStore::put_stream`].
/// S3's hard minimum is 5 MiB (except for the final part); picking
/// 8 MiB keeps the part count low for typical uploads while
/// honouring the lower bound. Configurable per-store on a future
/// pass; the constant lives here so a consumer can read the value
/// from the rustdoc.
const MULTIPART_CHUNK: usize = 8 * 1024 * 1024;

/// Explicit credentials path. Engine constructors that build their
/// own credential provider (e.g. wiring a `SecretStore`) reach for
/// [`S3BlobStore::open_with_credentials`] passing this struct.
#[derive(Clone, Debug)]
pub struct S3Credentials {
    /// Access-key id (`AWS_ACCESS_KEY_ID`).
    pub access_key_id: String,
    /// Secret access key (`AWS_SECRET_ACCESS_KEY`).
    pub secret_access_key: String,
    /// Optional session token for STS / IAM-role flows.
    pub session_token: Option<String>,
}

/// Construction-time configuration for an [`S3BlobStore`].
#[derive(Clone, Debug)]
pub struct S3BlobStoreConfig {
    /// Stable id reported by [`BlobStore::backend_id`]. Typically
    /// `s3:<region>:<bucket>`.
    pub backend_id: BackendId,
    /// Bucket name. The engine never exposes this on the trait
    /// surface (B1); it is engine-internal routing.
    pub bucket: String,
    /// AWS region. Required even for non-AWS endpoints — the SDK
    /// uses it to derive a SigV4 signing scope. `"garage"` or
    /// `"us-east-1"` are common picks for on-prem.
    pub region: String,
    /// Override the endpoint URL. `None` uses the AWS default for
    /// the region; pass `Some("http://garage:3900")` /
    /// `Some("http://minio:9000")` for self-hosted.
    pub endpoint_url: Option<String>,
    /// Use path-style addressing (`/bucket/key`) instead of
    /// virtual-hosted-style (`bucket.host/key`). **Required** for
    /// MinIO, Garage, and most on-prem S3-likes that cannot
    /// terminate TLS per bucket.
    pub force_path_style: bool,
    /// Default presign TTL hint. Per-call TTLs override on
    /// [`BlobStore::presign`].
    pub default_presign_ttl: Duration,
}

impl S3BlobStoreConfig {
    /// Build a config with the mandatory fields.
    pub fn new(
        backend_id: BackendId,
        bucket: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            backend_id,
            bucket: bucket.into(),
            region: region.into(),
            endpoint_url: None,
            force_path_style: false,
            default_presign_ttl: Duration::from_secs(900),
        }
    }

    /// Set [`S3BlobStoreConfig::endpoint_url`]; pass the full URL
    /// including scheme.
    pub fn endpoint_url(mut self, url: impl Into<String>) -> Self {
        self.endpoint_url = Some(url.into());
        self
    }

    /// Enable [`S3BlobStoreConfig::force_path_style`].
    pub fn force_path_style(mut self, on: bool) -> Self {
        self.force_path_style = on;
        self
    }
}

/// Errors specific to construction. Once built, the trait surface
/// returns [`BlobError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum S3BlobStoreError {
    /// Endpoint URL failed to parse.
    #[error("invalid endpoint URL: {0}")]
    InvalidEndpoint(String),
}

/// S3-backed blob store.
#[derive(Clone)]
pub struct S3BlobStore {
    client: Client,
    config: S3BlobStoreConfig,
}

impl S3BlobStore {
    /// Build using the SDK's default credential chain (env vars,
    /// shared config, IMDS, STS). Suitable for AWS deployments and
    /// any environment where the operator wants the SDK to discover
    /// creds the standard way.
    pub async fn open(config: S3BlobStoreConfig) -> Result<Self, S3BlobStoreError> {
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()));
        if let Some(ref url) = config.endpoint_url {
            loader = loader.endpoint_url(url.clone());
        }
        let shared = loader.load().await;
        let client = build_client(&shared, &config);
        Ok(Self { client, config })
    }

    /// Build with explicit credentials — typically pulled from a
    /// [`starter_spi::secrets::SecretStore`]. The two-constructor
    /// shape is deliberate: a consumer that types
    /// `S3BlobStore::open_with_credentials(...)` is opting in to
    /// "we manage the secret material," which is the right surface
    /// for Garage deployments where the access-key is minted per
    /// bucket on the admin API.
    pub async fn open_with_credentials(
        config: S3BlobStoreConfig,
        credentials: S3Credentials,
    ) -> Result<Self, S3BlobStoreError> {
        let creds = Credentials::new(
            credentials.access_key_id,
            credentials.secret_access_key,
            credentials.session_token,
            None,
            "starter-blob-s3",
        );
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(creds);
        if let Some(ref url) = config.endpoint_url {
            loader = loader.endpoint_url(url.clone());
        }
        let shared = loader.load().await;
        let client = build_client(&shared, &config);
        Ok(Self { client, config })
    }

    /// Borrow the SDK client. Crate-public so the Garage layering
    /// crate (`starter-blob-garage`) can share the same client for
    /// admin-API requests it presigns through SigV4.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Borrow the config so the Garage layer can read the bucket /
    /// region without re-typing.
    pub fn config(&self) -> &S3BlobStoreConfig {
        &self.config
    }

    fn mint_ref(&self, key: &str, etag: Etag, size: u64) -> BlobRef {
        BlobRef::mint(self.config.backend_id.clone(), key.to_owned(), etag, size)
    }
}

fn build_client(shared: &aws_config::SdkConfig, cfg: &S3BlobStoreConfig) -> Client {
    let mut builder: S3ConfigBuilder = aws_sdk_s3::config::Builder::from(shared);
    builder = builder.force_path_style(cfg.force_path_style);
    Client::from_conf(builder.build())
}

fn meta_from_head(out: &aws_sdk_s3::operation::head_object::HeadObjectOutput) -> BlobMeta {
    let size = out.content_length().unwrap_or(0).max(0) as u64;
    let etag = Etag::new(out.e_tag().unwrap_or_default().trim_matches('"'));
    let mut meta = BlobMeta::new(size, etag)
        .with_content_type(out.content_type().map(str::to_owned))
        .with_cache_control(out.cache_control().map(str::to_owned));
    if let Some(lm) = out.last_modified() {
        if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(lm.secs(), 0) {
            meta = meta.with_updated_at(Some(dt));
        }
    }
    meta
}

#[async_trait]
impl BlobStore for S3BlobStore {
    fn backend_id(&self) -> &BackendId {
        &self.config.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        debug!(target: TRACE_TARGET, key = %key, size = bytes.len(), "put_bytes");
        let size = bytes.len() as u64;
        let if_absent = opts.if_absent;
        let mut req = self
            .client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key.as_str())
            .body(ByteStream::from(bytes));
        if let Some(ct) = opts.content_type {
            req = req.content_type(ct);
        }
        if let Some(cc) = opts.cache_control {
            req = req.cache_control(cc);
        }
        if if_absent {
            // S3's standard `If-None-Match: *` semantic. Garage and
            // AWS both implement it; on a hit the server returns
            // 412, which our error mapper folds to
            // PreconditionFailed — but the SPI contract for
            // `if_absent` is `AlreadyExists`, so re-map below.
            req = req.if_none_match("*");
        }
        match req.send().await {
            Ok(out) => {
                let etag = Etag::new(out.e_tag().unwrap_or_default().trim_matches('"'));
                Ok(self.mint_ref(key.as_str(), etag, size))
            }
            Err(e) => Err(remap_if_absent(if_absent, map_sdk_error("put_object", e))),
        }
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        debug!(target: TRACE_TARGET, key = %key, "put_stream multipart");

        // Drain the first chunk to decide whether to multipart or
        // single-shot. S3 multipart has a per-part minimum of 5
        // MiB — for small uploads a single `PutObject` is cheaper
        // (one round-trip, one billing event).
        let mut s = stream;
        let mut buf = BytesMut::with_capacity(MULTIPART_CHUNK);
        let mut total: u64 = 0;
        while buf.len() < MULTIPART_CHUNK {
            match s.try_next().await? {
                Some(chunk) => {
                    total += chunk.len() as u64;
                    buf.extend_from_slice(&chunk);
                }
                None => break,
            }
        }

        // Peek the next chunk to know whether more parts follow.
        let peek = s.try_next().await?;
        if peek.is_none() {
            // Single-shot fits.
            return self.put_bytes(key, buf.freeze(), opts).await;
        }

        // Multipart path.
        let mut create_req = self
            .client
            .create_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key.as_str());
        if let Some(ct) = opts.content_type.clone() {
            create_req = create_req.content_type(ct);
        }
        if let Some(cc) = opts.cache_control.clone() {
            create_req = create_req.cache_control(cc);
        }
        let create: CreateMultipartUploadOutput = create_req
            .send()
            .await
            .map_err(|e| map_sdk_error("create_multipart_upload", e))?;
        let upload_id = create
            .upload_id()
            .ok_or_else(|| BlobError::backend(MultipartMissingId))?
            .to_owned();

        let mut parts: Vec<CompletedPart> = Vec::new();
        let mut part_number: i32 = 1;
        let mut current = buf;
        let mut next_chunk = peek;
        loop {
            // Drain follow-up chunks into `current` until it crosses
            // the MULTIPART_CHUNK threshold or the stream ends.
            while current.len() < MULTIPART_CHUNK {
                let next = match next_chunk.take() {
                    Some(c) => Some(c),
                    None => s.try_next().await?,
                };
                match next {
                    Some(c) => {
                        total += c.len() as u64;
                        current.extend_from_slice(&c);
                    }
                    None => break,
                }
            }

            let payload = current.split().freeze();
            let payload_len = payload.len();
            let up = match self
                .client
                .upload_part()
                .bucket(&self.config.bucket)
                .key(key.as_str())
                .upload_id(&upload_id)
                .part_number(part_number)
                .body(ByteStream::from(payload))
                .send()
                .await
            {
                Ok(o) => o,
                Err(e) => {
                    // Best-effort abort so we do not leak a half-
                    // open multipart upload on S3 billing.
                    let _ = self
                        .client
                        .abort_multipart_upload()
                        .bucket(&self.config.bucket)
                        .key(key.as_str())
                        .upload_id(&upload_id)
                        .send()
                        .await;
                    return Err(map_sdk_error("upload_part", e));
                }
            };
            parts.push(
                CompletedPart::builder()
                    .e_tag(up.e_tag().unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            );

            // Look for the next chunk to know if we're done.
            next_chunk = s.try_next().await?;
            if next_chunk.is_none() && current.is_empty() {
                break;
            }
            part_number += 1;
            // Guard against silly small final part: if stream is
            // done and we have a tail < 5MiB, that's fine — final
            // part can be smaller.
            let _ = payload_len; // suppress unused warn in release
        }

        let completed = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();
        let complete = self
            .client
            .complete_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key.as_str())
            .upload_id(&upload_id)
            .multipart_upload(completed)
            .send()
            .await
            .map_err(|e| map_sdk_error("complete_multipart_upload", e))?;

        let etag = Etag::new(complete.e_tag().unwrap_or_default().trim_matches('"'));
        Ok(self.mint_ref(key.as_str(), etag, total))
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let mut req = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(blob_ref.opaque_locator());
        if let Some(r) = range {
            let end = if r.end == u64::MAX {
                String::new()
            } else {
                r.end.to_string()
            };
            req = req.range(format!("bytes={}-{}", r.start, end));
        }
        let out = req
            .send()
            .await
            .map_err(|e| map_sdk_error("get_object", e))?;
        let body = out.body;
        // ByteStream -> futures::Stream<Item = Result<Bytes,_>>
        let s = futures::stream::unfold(body, |mut body| async move {
            match body.try_next().await {
                Ok(Some(chunk)) => Some((Ok(chunk), body)),
                Ok(None) => None,
                Err(e) => Some((Err(BlobError::backend(e)), body)),
            }
        });
        Ok(s.boxed())
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let out = self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(blob_ref.opaque_locator())
            .send()
            .await
            .map_err(|e| map_sdk_error("head_object", e))?;
        Ok(meta_from_head(&out))
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(blob_ref.opaque_locator())
            .send()
            .await
            .map(|_| ())
            .or_else(|e| match map_sdk_error("delete_object", e) {
                // S3 delete is documented idempotent: a missing key
                // returns 204, not 404. But some compat layers do
                // surface 404 — fold to Ok per the trait contract
                // ("delete is idempotent").
                BlobError::NotFound => Ok(()),
                other => Err(other),
            })
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        let mut req = self.client.list_objects_v2().bucket(&self.config.bucket);
        if let Some(p) = prefix {
            req = req.prefix(p.as_str());
        }
        if let Some(c) = cursor {
            req = req.continuation_token(c);
        }
        let out = req
            .send()
            .await
            .map_err(|e| map_sdk_error("list_objects_v2", e))?;
        let next = out.next_continuation_token().map(str::to_owned);
        let mut items = Vec::new();
        for obj in out.contents() {
            let Some(key) = obj.key() else { continue };
            let size = obj.size().unwrap_or(0).max(0) as u64;
            let etag = Etag::new(obj.e_tag().unwrap_or_default().trim_matches('"'));
            let meta = BlobMeta::new(size, etag.clone());
            let r = self.mint_ref(key, etag, size);
            items.push((r, meta));
        }
        Ok(ListPage::new(items, next))
    }

    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        let cfg = PresigningConfig::expires_in(ttl).map_err(BlobError::backend)?;
        let url_str = match op {
            PresignOp::Get => self
                .client
                .get_object()
                .bucket(&self.config.bucket)
                .key(blob_ref.opaque_locator())
                .presigned(cfg)
                .await
                .map_err(|e| map_sdk_error("presign_get", e))?
                .uri()
                .to_string(),
            PresignOp::Put => self
                .client
                .put_object()
                .bucket(&self.config.bucket)
                .key(blob_ref.opaque_locator())
                .presigned(cfg)
                .await
                .map_err(|e| map_sdk_error("presign_put", e))?
                .uri()
                .to_string(),
            // `PresignOp` is `#[non_exhaustive]`; an unknown variant
            // surfaces as Unsupported per the trait's B3 contract.
            _ => return Err(BlobError::Unsupported),
        };
        Ok(PresignedUrl {
            url: url_str,
            method: op,
            expires_at: SystemTime::now() + ttl,
        })
    }

    async fn copy_server_side(
        &self,
        src: &BlobRef,
        dst_key: &BlobKey,
    ) -> Result<BlobRef, BlobError> {
        // S3 CopyObject is bucket-scoped on this engine — copying
        // *between* engines must go through `copy_via_client` per
        // B3 (the consumer is the one paying the bytes-through-
        // process cost, so they have to type it out).
        if src.backend_id() != self.backend_id() {
            return Err(BlobError::Unsupported);
        }
        let copy_source = format!("{}/{}", self.config.bucket, src.opaque_locator());
        let out = self
            .client
            .copy_object()
            .bucket(&self.config.bucket)
            .key(dst_key.as_str())
            .copy_source(copy_source)
            .send()
            .await
            .map_err(|e| map_sdk_error("copy_object", e))?;
        let etag = Etag::new(
            out.copy_object_result()
                .and_then(|r| r.e_tag())
                .unwrap_or_default()
                .trim_matches('"'),
        );
        let size = self
            .head(&self.mint_ref(dst_key.as_str(), etag.clone(), 0))
            .await?
            .size;
        Ok(self.mint_ref(dst_key.as_str(), etag, size))
    }
}

/// Tiny helper for the multipart unhappy path so the `?` on a
/// missing upload-id surface stays an honest typed error.
#[derive(Debug, thiserror::Error)]
#[error("create_multipart_upload returned no upload id")]
struct MultipartMissingId;

/// When the caller set `if_absent` (which we mapped to
/// `If-None-Match: *`), S3 returns 412 → PreconditionFailed.
/// But the SPI contract for `if_absent` is `AlreadyExists`. Re-map.
fn remap_if_absent(was_if_absent: bool, err: BlobError) -> BlobError {
    if was_if_absent {
        if let BlobError::PreconditionFailed = err {
            return BlobError::AlreadyExists;
        }
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builders_compose() {
        let cfg = S3BlobStoreConfig::new(BackendId::new("s3:test"), "bkt", "us-east-1")
            .endpoint_url("http://localhost:9000")
            .force_path_style(true);
        assert!(cfg.force_path_style);
        assert_eq!(cfg.endpoint_url.as_deref(), Some("http://localhost:9000"));
    }

    #[test]
    fn remap_if_absent_only_when_flag_set() {
        assert!(matches!(
            remap_if_absent(true, BlobError::PreconditionFailed),
            BlobError::AlreadyExists
        ));
        assert!(matches!(
            remap_if_absent(false, BlobError::PreconditionFailed),
            BlobError::PreconditionFailed
        ));
    }

    /// The trait must be object-safe so consumers can store
    /// `Arc<dyn BlobStore>` per the SwapTest in the SCOPE. A failure
    /// here is a compile-time error rather than a test failure.
    #[allow(dead_code)]
    fn _object_safety(_: &dyn BlobStore) {}
}
