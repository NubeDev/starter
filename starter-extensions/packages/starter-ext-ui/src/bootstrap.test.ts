// `bootstrapExtensions` unit tests.
//
// Asserts the helper:
// 1. Walks the list endpoint and skips entries marked `enabled: false`.
// 2. Filters out manifests with no `contributes.ui`.
// 3. Builds the correct remoteEntry URL (basePath + id + ui/entry).
// 4. Registers each surviving remote on the host manager.
// 5. Isolates per-extension failures so one bad remote does not abort
//    the loop.

import { describe, expect, it, vi } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { bootstrapExtensions } from "./bootstrap.js";
import { ExtensionHostManager, type ExtensionRemoteFactory } from "./host-manager.js";

interface Route {
  url: string;
  body: unknown;
  status?: number;
}

/** Stub `fetch` whose responses are looked up by URL. */
function fakeFetch(routes: Route[]): typeof fetch {
  const byUrl = new Map(routes.map((r) => [r.url, r]));
  return async function fakeFetch(input: RequestInfo | URL): Promise<Response> {
    const url = typeof input === "string" ? input : input.toString();
    const r = byUrl.get(url);
    if (!r) {
      return new Response(null, { status: 404 });
    }
    return new Response(JSON.stringify(r.body), {
      status: r.status ?? 200,
      headers: { "content-type": "application/json" },
    });
  } as unknown as typeof fetch;
}

function makeManager(fetchImpl: typeof fetch): ExtensionHostManager {
  return new ExtensionHostManager({
    client: new StarterClient({ baseUrl: "http://host", fetch: fetchImpl }),
    singletons: {
      react: { version: "18.3.1", instance: { __id: "react" } },
    },
  });
}

const ECHO_FACTORY: ExtensionRemoteFactory = {
  singletons: { react: { version: "18.3.1" } },
  init(handle) {
    handle.register({ Panel: () => null as never });
  },
};

describe("bootstrapExtensions", () => {
  it("registers every enabled extension that contributes a UI", async () => {
    const fetchImpl = fakeFetch([
      {
        url: "http://host/api/v1/extensions",
        body: [
          { id: "com.acme.a", enabled: true },
          { id: "com.acme.b", enabled: true },
        ],
      },
      {
        url: "http://host/api/v1/extensions/com.acme.a",
        body: {
          id: "com.acme.a",
          enabled: "enabled",
          state: "validated",
          manifest: {
            id: "com.acme.a",
            version: "0.1.0",
            contributes: {
              ui: {
                entry: "ui/remoteEntry.js",
                exposes: [
                  { name: "Panel", module: "./Panel", slot: "sidebar" },
                ],
              },
            },
          },
        },
      },
      {
        url: "http://host/api/v1/extensions/com.acme.b",
        body: {
          id: "com.acme.b",
          enabled: "enabled",
          state: "validated",
          // No `contributes.ui` — should be skipped.
          manifest: { id: "com.acme.b", version: "0.1.0", contributes: {} },
        },
      },
    ]);
    const mgr = makeManager(fetchImpl);
    const importRemote = vi.fn(async () => ECHO_FACTORY);

    const result = await bootstrapExtensions(mgr, {
      basePath: "/api/v1/extensions",
      importRemote,
    });

    expect(result).toEqual({
      seen: 2,
      skippedNoUi: 1,
      skippedDisabled: 0,
      registered: 1,
      failed: 0,
    });
    expect(importRemote).toHaveBeenCalledWith(
      "http://host/api/v1/extensions/com.acme.a/ui/ui/remoteEntry.js",
    );
    expect(mgr.getRemote("com.acme.a")).toBeDefined();
    expect(mgr.getRemote("com.acme.b")).toBeUndefined();
  });

  it("skips entries the server reports as disabled", async () => {
    const fetchImpl = fakeFetch([
      {
        url: "http://host/extensions",
        body: { extensions: [{ id: "com.acme.disabled", enabled: false }] },
      },
    ]);
    const mgr = makeManager(fetchImpl);
    const importRemote = vi.fn();

    const result = await bootstrapExtensions(mgr, { importRemote });

    expect(result.skippedDisabled).toBe(1);
    expect(result.registered).toBe(0);
    expect(importRemote).not.toHaveBeenCalled();
  });

  it("isolates per-extension failures and continues with the rest", async () => {
    const fetchImpl = fakeFetch([
      {
        url: "http://host/extensions",
        body: [
          { id: "com.acme.bad", enabled: true },
          { id: "com.acme.good", enabled: true },
        ],
      },
      {
        url: "http://host/extensions/com.acme.bad",
        body: {
          id: "com.acme.bad",
          enabled: "enabled",
          state: "validated",
          manifest: {
            id: "com.acme.bad",
            contributes: {
              ui: { entry: "ui/remoteEntry.js", exposes: [] },
            },
          },
        },
      },
      {
        url: "http://host/extensions/com.acme.good",
        body: {
          id: "com.acme.good",
          enabled: "enabled",
          state: "validated",
          manifest: {
            id: "com.acme.good",
            contributes: {
              ui: { entry: "ui/remoteEntry.js", exposes: [] },
            },
          },
        },
      },
    ]);
    const mgr = makeManager(fetchImpl);
    const errors: string[] = [];
    const importRemote = vi.fn(async (url: string) => {
      if (url.includes("com.acme.bad")) {
        throw new Error("simulated remoteEntry boom");
      }
      return ECHO_FACTORY;
    });

    const result = await bootstrapExtensions(mgr, {
      importRemote,
      onError: (id) => errors.push(id),
    });

    expect(result.failed).toBe(1);
    expect(result.registered).toBe(1);
    expect(errors).toEqual(["com.acme.bad"]);
    expect(mgr.getRemote("com.acme.good")).toBeDefined();
  });
});
