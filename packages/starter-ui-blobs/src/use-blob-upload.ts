// `useBlobUpload` — locked hook surface for direct-PUT uploads
// against any starter-shaped backend.
//
// See [`./types.ts`] for the `PresignResponse` shape the
// consumer's `presignEndpoint` must return.

import { useCallback, useRef, useState } from "react";

import { metaKeys, type BlobMeta, type BlobRef, type PresignResponse } from "./types.js";

/**
 * Options for [`useBlobUpload`].
 *
 * The hook is taxonomy-agnostic: it does not know what a
 * "project", "user", or "page" is. The consumer's
 * `presignEndpoint` is the only thing that binds an upload to a
 * domain object — that endpoint authenticates the caller, decides
 * the target scope, and mints a presigned `PUT` against the
 * appropriate `Namespaced(BlobStore)`.
 */
export interface UseBlobUploadOptions {
  /**
   * URL the hook `POST`s a JSON `{ filename, contentType, size }`
   * to. The endpoint replies with [`PresignResponse`].
   *
   * Path semantics are consumer-defined; common shapes are
   * `/api/projects/:id/blobs/presign` or `/api/blobs/presign`
   * with auth state implied by cookie.
   */
  presignEndpoint: string;

  /**
   * Called after the `PUT` succeeds. Receives the durable
   * [`BlobRef`] (commit it to your domain row) and any
   * [`BlobMeta`] the presign endpoint chose to surface.
   */
  onUploaded: (ref: BlobRef, meta: BlobMeta) => void;

  /**
   * Hard cap on body size, enforced *before* the presign
   * round-trip — saves a wasted server call for files that will
   * be rejected. Server-side enforcement still happens in the
   * presign endpoint and at the engine layer (see "quotas" in
   * the storage scope, planned for 0.2).
   */
  maxBytes?: number;

  /**
   * Allowed MIME types. When omitted, every type passes. Matching
   * is case-insensitive and supports a trailing `/*` wildcard
   * (e.g. `["image/*", "application/pdf"]`).
   */
  acceptedTypes?: string[];

  /**
   * Override the `fetch` implementation. Lets tests inject a
   * deterministic transport; in production this defaults to the
   * global `fetch`.
   */
  fetchImpl?: typeof fetch;
}

/**
 * Return value of [`useBlobUpload`]. Designed so the call site
 * does not need an effect to wire progress / error state — both
 * are reactive React state.
 */
export interface UseBlobUploadResult {
  /**
   * Upload `file` end-to-end: validate → presign → `PUT` →
   * resolve to the durable [`BlobRef`]. Validation and transport
   * failures populate `error` and reject the returned promise —
   * pick whichever suits your call site.
   */
  upload: (file: File) => Promise<BlobRef>;

  /**
   * Fractional progress of the in-flight `PUT`, `0..=1`, or
   * `null` when no upload is active. Browsers only emit progress
   * for `XMLHttpRequest`-style transports; the hook uses `fetch`
   * by default, so this currently reports `0` at start and `1`
   * on completion. A future revision may switch to XHR for
   * granular progress without changing the surface.
   */
  progress: number | null;

  /** Last error from `upload`, or `null` if the last call succeeded or none has run. */
  error: Error | null;
}

/**
 * Error raised when validation (size / type) fails *before* the
 * presign round-trip. Distinguished by name so call sites can
 * surface a user-friendly message without parsing the description.
 */
export class BlobUploadValidationError extends Error {
  constructor(
    message: string,
    public readonly kind: "size" | "type",
  ) {
    super(message);
    this.name = "BlobUploadValidationError";
  }
}

/**
 * Error raised when the presign endpoint or the subsequent `PUT`
 * fails. `status` is the HTTP status from whichever request
 * failed (presign or PUT); `phase` distinguishes them.
 */
export class BlobUploadTransportError extends Error {
  constructor(
    message: string,
    public readonly phase: "presign" | "put",
    public readonly status: number,
  ) {
    super(message);
    this.name = "BlobUploadTransportError";
  }
}

export function useBlobUpload(opts: UseBlobUploadOptions): UseBlobUploadResult {
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<Error | null>(null);

  // `opts` is captured by reference rather than dependency-tracked
  // so consumers can mutate `onUploaded` between renders without
  // causing the returned `upload` callback to re-allocate (which
  // would invalidate any memoized parent).
  const optsRef = useRef(opts);
  optsRef.current = opts;

  const upload = useCallback(async (file: File): Promise<BlobRef> => {
    const cur = optsRef.current;
    setError(null);
    setProgress(0);

    try {
      validate(file, cur);

      const presign = await requestPresign(file, cur);
      await putBody(file, presign, cur);

      setProgress(1);

      const meta = presign.meta ?? metaFromFile(file, presign.ref);
      cur.onUploaded(presign.ref, meta);
      return presign.ref;
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      setError(err);
      setProgress(null);
      throw err;
    }
  }, []);

  return { upload, progress, error };
}

function validate(file: File, opts: UseBlobUploadOptions): void {
  if (opts.maxBytes !== undefined && file.size > opts.maxBytes) {
    throw new BlobUploadValidationError(
      `file is ${file.size} bytes, exceeds ${opts.maxBytes}-byte cap`,
      "size",
    );
  }
  if (opts.acceptedTypes !== undefined && opts.acceptedTypes.length > 0) {
    if (!typeAccepted(file.type, opts.acceptedTypes)) {
      throw new BlobUploadValidationError(
        `content-type "${file.type}" is not in the accepted list`,
        "type",
      );
    }
  }
}

function typeAccepted(actual: string, accepted: string[]): boolean {
  const a = actual.toLowerCase();
  return accepted.some((rule) => {
    const r = rule.toLowerCase();
    if (r.endsWith("/*")) {
      // keep trailing `/` so `image/` does not match `imagejpeg`
      const prefix = r.slice(0, -1);
      return a.startsWith(prefix);
    }
    return a === r;
  });
}

async function requestPresign(
  file: File,
  opts: UseBlobUploadOptions,
): Promise<PresignResponse> {
  const f = opts.fetchImpl ?? fetch;
  const resp = await f(opts.presignEndpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      filename: file.name,
      contentType: file.type || "application/octet-stream",
      size: file.size,
    }),
  });
  if (!resp.ok) {
    throw new BlobUploadTransportError(
      `presign endpoint returned ${resp.status}`,
      "presign",
      resp.status,
    );
  }
  return (await resp.json()) as PresignResponse;
}

async function putBody(
  file: File,
  presign: PresignResponse,
  opts: UseBlobUploadOptions,
): Promise<void> {
  const f = opts.fetchImpl ?? fetch;
  const resp = await f(presign.url, {
    method: "PUT",
    headers: presign.headers,
    body: file,
  });
  if (!resp.ok) {
    throw new BlobUploadTransportError(
      `PUT failed with ${resp.status}`,
      "put",
      resp.status,
    );
  }
}

function metaFromFile(file: File, ref: BlobRef): BlobMeta {
  return {
    size: ref.size,
    etag: ref.etag,
    content_type: file.type || undefined,
    user_metadata: {
      [metaKeys.FILENAME]: file.name,
    },
  };
}
