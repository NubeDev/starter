// TypeScript counterparts to the Rust types in
// `starter-spi::blob`. These are *not* re-derived from a schema —
// they mirror the serde-stable wire shape the Rust side already
// commits to. Changing either side without changing the other is
// a protocol break.

/**
 * Opaque, durable handle for a stored blob.
 *
 * Mirrors `starter_spi::blob::BlobRef`. Treat as opaque from
 * frontend code: persist it in domain rows, hand it back to
 * download URLs, never inspect `opaque_locator` or build a path
 * out of `backend_id`. That is what makes a `Namespaced`/`Tiered`
 * swap on the Rust side non-breaking for content already stored
 * in markdown bodies.
 *
 * Field names use the Rust snake_case originals so the JSON
 * envelope round-trips unchanged through `JSON.stringify` /
 * `serde_json::from_slice` without a transformer.
 */
export interface BlobRef {
  /** Engine instance that minted this ref. */
  backend_id: string;
  /** Engine-defined routing token. Opaque to consumers. */
  opaque_locator: string;
  /** Version marker; changes on overwrite. */
  etag: string;
  /** Size in bytes. */
  size: number;
}

/**
 * Observable metadata for a stored blob.
 *
 * Mirrors `starter_spi::blob::BlobMeta`. Returned by the presign
 * endpoint alongside the freshly minted [`BlobRef`] so the caller
 * can populate UI state without an extra `HEAD` round-trip.
 */
export interface BlobMeta {
  size: number;
  etag: string;
  content_type?: string | null;
  cache_control?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  /**
   * Free-form, consumer-defined string→string metadata. Use the
   * constants in [`metaKeys`] for portable spellings of reserved
   * keys (`filename`, `uploaded_by`, `uploaded_at`).
   */
  user_metadata?: Record<string, string>;
}

/**
 * Reserved keys in [`BlobMeta.user_metadata`]. Mirrors
 * `starter_spi::blob::meta_keys` on the Rust side: every starter
 * consumer agrees on these spellings so a `BlobRef` is portable
 * across consumers.
 */
export const metaKeys = {
  /** Original client-supplied filename, UTF-8. */
  FILENAME: "filename",
  /** Opaque consumer-defined principal id. */
  UPLOADED_BY: "uploaded_by",
  /** RFC3339 timestamp of the upload. */
  UPLOADED_AT: "uploaded_at",
} as const;

/**
 * Shape returned by a consumer's `presignEndpoint`. The endpoint
 * is consumer-defined (it's where domain-level authz happens),
 * but its response shape is locked here so [`useBlobUpload`] can
 * speak to any starter-shaped backend.
 *
 * The endpoint typically:
 *
 * 1. Authenticates the caller and decides whether they may upload
 *    to the target scope (project, conversation, …).
 * 2. Allocates a `BlobKey` under the appropriate namespace.
 * 3. Calls `BlobStore::presign(blob_ref, PresignOp::Put, ttl)`
 *    against the configured store.
 * 4. Returns the resulting `{ url, headers }` plus a placeholder
 *    [`BlobRef`] the frontend will commit on upload success.
 *
 * `headers` is an object the client merges into the `PUT`
 * (typically `content-type` and any signed auth headers the
 * engine requires).
 */
export interface PresignResponse {
  /** Pre-signed URL the client `PUT`s the body to. */
  url: string;
  /** Headers the client must echo on the `PUT`. */
  headers: Record<string, string>;
  /** Durable handle to commit once the `PUT` succeeds. */
  ref: BlobRef;
  /** Optional metadata to surface on `onUploaded` without `HEAD`. */
  meta?: BlobMeta;
}
