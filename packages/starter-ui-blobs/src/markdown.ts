// `useBlobUploadForMarkdown` — markdown-editor adapter that
// composes [`useBlobUpload`] into a single `onImageUpload(file)`
// callback compatible with `@uiw/react-md-editor`, tiptap's
// paste-image extension, codemirror's drop handler, etc.
//
// Why this lives behind a subpath import (`@nube/starter-ui-blobs/markdown`):
//
// Markdown editors all want the same shape — *"hand me an async
// function that takes a `File` and resolves to a string URL the
// editor will inline into the body"*. If every consumer wires
// `useBlobUpload` themselves they're tempted to inline the
// engine's presigned `PUT` URL (TTL-bound) or the raw key
// (B2-violating). This adapter forces them to inline a proxy URL
// minted from the durable `BlobRef`, which is what keeps a later
// `Namespaced`/`Tiered` swap non-breaking for markdown rows
// already in the database.
//
// Kept in a separate file (and subpath export) so consumers who
// only upload images-as-attachments — without inlining into a
// markdown body — don't import a code path they will never run.

import { useCallback, useRef } from "react";

import { useBlobUpload, type UseBlobUploadOptions } from "./use-blob-upload.js";
import type { BlobRef } from "./types.js";

/**
 * Options for [`useBlobUploadForMarkdown`].
 *
 * `presignEndpoint`, `maxBytes`, `acceptedTypes`, `fetchImpl`
 * pass through to [`useBlobUpload`] unchanged.
 *
 * `proxyUrlFor` is the load-bearing piece: it turns a durable
 * [`BlobRef`] into the URL the markdown body should embed.
 * Typically it routes to your `starter-blob-axum` proxy mount
 * (e.g. ``(ref) => `/api/blobs/${encodeRef(ref)}` ``). The adapter
 * deliberately requires the consumer to supply this rather than
 * defaulting to the presigned `PUT`'s URL — see file header.
 */
export interface UseBlobUploadForMarkdownOptions
  extends Omit<UseBlobUploadOptions, "onUploaded"> {
  /**
   * Build the URL the markdown body will embed. Should produce
   * a stable, auth-checked GET endpoint, **not** a presigned URL
   * (which would expire) and **not** an engine key (which would
   * couple the rendered body to today's engine choice).
   */
  proxyUrlFor: (ref: BlobRef) => string;

  /**
   * Optional callback fired in addition to inlining — useful for
   * persisting the `BlobRef` to a side table (e.g. "attachments
   * on this page") so deletes can later reach the engine even
   * when the markdown referencing it has been edited.
   */
  onUploaded?: (ref: BlobRef) => void;
}

/**
 * Returns an `onImageUpload(file)` callback that resolves to the
 * URL the markdown editor should embed. The hook's other state
 * (`progress`, `error`) is exposed via [`useBlobUpload`] for
 * call sites that want it; this adapter is intentionally narrow.
 */
export function useBlobUploadForMarkdown(
  opts: UseBlobUploadForMarkdownOptions,
): (file: File) => Promise<string> {
  // `proxyUrlFor` is captured by ref so an inline arrow at the
  // call site doesn't remount the underlying upload hook on
  // every render.
  const proxyUrlForRef = useRef(opts.proxyUrlFor);
  proxyUrlForRef.current = opts.proxyUrlFor;

  const { upload } = useBlobUpload({
    presignEndpoint: opts.presignEndpoint,
    maxBytes: opts.maxBytes,
    acceptedTypes: opts.acceptedTypes,
    fetchImpl: opts.fetchImpl,
    onUploaded: (ref) => {
      opts.onUploaded?.(ref);
    },
  });

  return useCallback(
    async (file: File) => {
      const ref = await upload(file);
      return proxyUrlForRef.current(ref);
    },
    [upload],
  );
}
