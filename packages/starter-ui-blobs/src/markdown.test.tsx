// Behavioural test for [`useBlobUploadForMarkdown`]: the markdown
// adapter must inline the URL returned by `proxyUrlFor(ref)`,
// **not** the engine's presigned PUT URL. This is the load-bearing
// guarantee that keeps a later `Namespaced`/`Tiered` swap
// non-breaking for markdown rows already in the database.

import { describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";

import { useBlobUploadForMarkdown } from "./markdown.js";
import type { BlobRef } from "./types.js";

function makeRef(): BlobRef {
  return { backend_id: "mem", opaque_locator: "loc-1", etag: "e", size: 1 };
}

describe("useBlobUploadForMarkdown", () => {
  it("returns the proxyUrlFor(ref) URL, not the presign PUT URL", async () => {
    const ref = makeRef();
    const fetchImpl = (async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      if (url === "/presign") {
        return new Response(
          JSON.stringify({
            url: "https://upload.example/PRESIGNED-DO-NOT-INLINE",
            headers: {},
            ref,
          }),
          { status: 200 },
        );
      }
      return new Response("{}", { status: 200 });
    }) as typeof fetch;

    const proxyUrlFor = vi.fn((r: BlobRef) => `/api/blobs/${r.etag}`);
    const onUploaded = vi.fn();

    const { result } = renderHook(() =>
      useBlobUploadForMarkdown({
        presignEndpoint: "/presign",
        proxyUrlFor,
        onUploaded,
        fetchImpl,
      }),
    );

    const file = new File(["x"], "x.png", { type: "image/png" });
    const url = await result.current(file);

    expect(url).toBe("/api/blobs/e");
    expect(url).not.toContain("PRESIGNED");
    expect(proxyUrlFor).toHaveBeenCalledWith(ref);
    expect(onUploaded).toHaveBeenCalledWith(ref);
  });
});
