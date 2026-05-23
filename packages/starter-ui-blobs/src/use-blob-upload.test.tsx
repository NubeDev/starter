// Behavioural tests for [`useBlobUpload`]:
// - happy-path presign → PUT → onUploaded
// - size cap rejection before any network call
// - mime-type rejection (exact + wildcard)
// - presign 4xx surfaces as BlobUploadTransportError(phase=presign)
// - PUT 4xx surfaces as BlobUploadTransportError(phase=put)
// - progress + error state transitions track the upload lifecycle

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";

import {
  BlobUploadTransportError,
  BlobUploadValidationError,
  useBlobUpload,
} from "./use-blob-upload.js";
import type { BlobRef, PresignResponse } from "./types.js";

afterEach(() => {
  cleanup();
});

function makeRef(over: Partial<BlobRef> = {}): BlobRef {
  return {
    backend_id: "mem",
    opaque_locator: "loc-1",
    etag: "etag-1",
    size: 4,
    ...over,
  };
}

function makeFile(name: string, body: string, type = "text/plain"): File {
  return new File([body], name, { type });
}

/**
 * Build a fetch double that returns scripted responses per URL.
 * Each entry is consumed once in order so we can assert call
 * ordering as a side effect of `expect(fetch).toHaveBeenCalled…`.
 */
function scriptFetch(
  routes: Array<{
    when: (url: string, init: RequestInit) => boolean;
    body: unknown;
    status?: number;
  }>,
): typeof fetch {
  let i = 0;
  return (async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const route = routes[i++];
    if (!route) throw new Error(`unexpected fetch to ${url}`);
    if (!route.when(url, init)) {
      throw new Error(`route ${i - 1} did not match url=${url} method=${init.method}`);
    }
    return new Response(JSON.stringify(route.body), { status: route.status ?? 200 });
  }) as typeof fetch;
}

describe("useBlobUpload", () => {
  it("uploads happy path and reports the BlobRef from the presign response", async () => {
    const ref = makeRef({ size: 5 });
    const presign: PresignResponse = {
      url: "https://upload.example/blob-1",
      headers: { "content-type": "text/plain" },
      ref,
    };
    const onUploaded = vi.fn();
    const fetchImpl = scriptFetch([
      {
        when: (url, init) => url === "/presign" && init.method === "POST",
        body: presign,
      },
      {
        when: (url, init) => url === presign.url && init.method === "PUT",
        body: {},
      },
    ]);

    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded,
        fetchImpl,
      }),
    );

    expect(result.current.progress).toBeNull();

    await act(async () => {
      const got = await result.current.upload(makeFile("hello.txt", "hello"));
      expect(got).toEqual(ref);
    });

    expect(onUploaded).toHaveBeenCalledWith(
      ref,
      expect.objectContaining({
        user_metadata: expect.objectContaining({ filename: "hello.txt" }),
      }),
    );
    expect(result.current.progress).toBe(1);
    expect(result.current.error).toBeNull();
  });

  it("rejects files over maxBytes without calling the presign endpoint", async () => {
    const fetchImpl = vi.fn();
    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded: () => {},
        maxBytes: 3,
        fetchImpl: fetchImpl as unknown as typeof fetch,
      }),
    );

    await act(async () => {
      await expect(
        result.current.upload(makeFile("big.txt", "abcdef")),
      ).rejects.toBeInstanceOf(BlobUploadValidationError);
    });

    expect(fetchImpl).not.toHaveBeenCalled();
    expect(result.current.error).toBeInstanceOf(BlobUploadValidationError);
  });

  it("matches acceptedTypes with a trailing /* wildcard", async () => {
    const ref = makeRef();
    const fetchImpl = scriptFetch([
      { when: (url) => url === "/presign", body: { url: "https://u/x", headers: {}, ref } },
      { when: (url) => url === "https://u/x", body: {} },
    ]);
    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded: () => {},
        acceptedTypes: ["image/*"],
        fetchImpl,
      }),
    );

    await act(async () => {
      await result.current.upload(makeFile("a.png", "PNG", "image/png"));
    });

    expect(result.current.error).toBeNull();
  });

  it("rejects mime types not in acceptedTypes", async () => {
    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded: () => {},
        acceptedTypes: ["image/*"],
        fetchImpl: vi.fn() as unknown as typeof fetch,
      }),
    );

    await act(async () => {
      await expect(
        result.current.upload(makeFile("a.txt", "x", "text/plain")),
      ).rejects.toMatchObject({ kind: "type" });
    });
  });

  it("surfaces presign-endpoint failures as BlobUploadTransportError(phase=presign)", async () => {
    const fetchImpl = (async () =>
      new Response("nope", { status: 403 })) as typeof fetch;
    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded: () => {},
        fetchImpl,
      }),
    );

    await act(async () => {
      await expect(result.current.upload(makeFile("f.txt", "x")))
        .rejects.toMatchObject({ phase: "presign", status: 403 });
    });
    expect(result.current.error).toBeInstanceOf(BlobUploadTransportError);
  });

  it("surfaces PUT failures as BlobUploadTransportError(phase=put)", async () => {
    const ref = makeRef();
    const fetchImpl = scriptFetch([
      { when: (url) => url === "/presign", body: { url: "https://u/x", headers: {}, ref } },
      { when: (url) => url === "https://u/x", body: "boom", status: 500 },
    ]);
    const { result } = renderHook(() =>
      useBlobUpload({
        presignEndpoint: "/presign",
        onUploaded: () => {},
        fetchImpl,
      }),
    );

    await act(async () => {
      await expect(result.current.upload(makeFile("f.txt", "x")))
        .rejects.toMatchObject({ phase: "put", status: 500 });
    });
  });
});
