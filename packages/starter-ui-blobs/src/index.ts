// `@nube/starter-ui-blobs` — direct-PUT upload hook + types for
// the starter blob-storage seam. The markdown-editor adapter
// lives at `@nube/starter-ui-blobs/markdown` so consumers who
// never inline blobs into markdown bodies don't import that code
// path.
//
// Matching Rust crates:
// - `starter-spi::blob` — `BlobRef`, `BlobMeta`, `meta_keys`.
// - `starter-blob-axum` — authenticated GET proxy the
//   `proxyUrlFor` (in the markdown subpath) typically routes to.

export {
  useBlobUpload,
  BlobUploadValidationError,
  BlobUploadTransportError,
} from "./use-blob-upload.js";
export type {
  UseBlobUploadOptions,
  UseBlobUploadResult,
} from "./use-blob-upload.js";

export { metaKeys } from "./types.js";
export type { BlobRef, BlobMeta, PresignResponse } from "./types.js";
