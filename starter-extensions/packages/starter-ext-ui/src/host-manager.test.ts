// `ExtensionHostManager` unit tests.
//
// Heavy on singleton negotiation because that is the single biggest
// load-bearing rule for the host runtime (SCOPE.md
// §"Singleton-mismatch handling (UI)" — load-time refusal).

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import {
  ExtensionHostManager,
  type ExtensionRemoteFactory,
  type ManifestUi,
  type SingletonProvision,
} from "./host-manager.js";
import { SingletonMismatchError } from "./singletons.js";

// One fixed React-shaped singleton both extensions in the
// "no React duplication" scenario share. Concretely it's any unique
// reference — what matters is that *both* `init` calls receive the
// *same* reference, which is the only thing that prevents bundlers
// from compiling two React copies into the page.
const HOST_REACT = { __id: "the-one-react" } as const;
const HOST_REACT_DOM = { __id: "the-one-react-dom" } as const;

function makeManager(
  singletons: Record<string, SingletonProvision> = {
    react: { version: "18.3.1", instance: HOST_REACT },
    "react-dom": { version: "18.3.1", instance: HOST_REACT_DOM },
  },
): ExtensionHostManager {
  return new ExtensionHostManager({
    client: new StarterClient({ baseUrl: "http://localhost.invalid" }),
    singletons,
  });
}

const UI: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "Panel", module: "./Panel", slot: "sidebar" }],
};

describe("registerExtensionRemote (singleton negotiation)", () => {
  it("passes the host's singleton instances to the extension's init", async () => {
    const mgr = makeManager();
    let received: unknown = undefined;
    const factory: ExtensionRemoteFactory = {
      singletons: { react: { version: "18.3.1" } },
      init(handle) {
        received = handle.singletons.react;
      },
    };
    await mgr.registerExtensionRemote("com.acme.a", UI, factory);
    expect(received).toBe(HOST_REACT);
  });

  it("refuses a remote with a major-version mismatch", async () => {
    const mgr = makeManager();
    const factory: ExtensionRemoteFactory = {
      singletons: { react: { version: "19.0.0" } },
      init() {
        throw new Error("init must not run on mismatch");
      },
    };
    await expect(
      mgr.registerExtensionRemote("com.acme.bad", UI, factory),
    ).rejects.toBeInstanceOf(SingletonMismatchError);
    expect(mgr.getRemote("com.acme.bad")).toBeUndefined();
  });

  it("refuses a remote requesting a singleton the host does not provide", async () => {
    const mgr = makeManager();
    const factory: ExtensionRemoteFactory = {
      singletons: { "made-up-lib": { version: "1.0.0" } },
      init() {
        throw new Error("init must not run on mismatch");
      },
    };
    await expect(
      mgr.registerExtensionRemote("com.acme.unknown", UI, factory),
    ).rejects.toBeInstanceOf(SingletonMismatchError);
  });

  it("notifies subscribers on registration", async () => {
    const mgr = makeManager();
    let ticks = 0;
    mgr.subscribe(() => {
      ticks++;
    });
    await mgr.registerExtensionRemote("com.acme.a", UI, {
      singletons: { react: { version: "18.0.0" } },
      init() {},
    });
    expect(ticks).toBe(1);
    mgr.unregisterExtensionRemote("com.acme.a");
    expect(ticks).toBe(2);
  });
});

describe("the two-extensions-no-React-duplication smoke test", () => {
  it("hands both extensions the same React reference (no duplication)", async () => {
    const mgr = makeManager();

    let aReact: unknown = null;
    let bReact: unknown = null;

    await mgr.registerExtensionRemote("com.acme.a", UI, {
      singletons: {
        react: { version: "18.3.1" },
        "react-dom": { version: "18.3.0" },
      },
      init(handle) {
        aReact = handle.singletons.react;
      },
    });
    await mgr.registerExtensionRemote("com.acme.b", UI, {
      singletons: {
        react: { version: "18.0.0" },
        "react-dom": { version: "18.2.0" },
      },
      init(handle) {
        bReact = handle.singletons.react;
      },
    });

    // Reference equality is the test. If a bundler ever loaded two
    // copies of React this would fail; the host hands out exactly
    // the one instance it itself holds.
    expect(aReact).toBe(HOST_REACT);
    expect(bReact).toBe(HOST_REACT);
    expect(aReact).toBe(bReact);

    // And the host's singleton table did not grow — still one entry
    // per shared pkg.
    expect(Object.keys(mgr.singletons).sort()).toEqual(
      ["react", "react-dom"].sort(),
    );
  });
});

describe("resolveSlot", () => {
  it("maps exposed components to the requested slot in source order", async () => {
    const mgr = makeManager();
    const ComponentA = () => null;
    const ComponentB = () => null;

    await mgr.registerExtensionRemote(
      "com.acme.a",
      {
        entry: "ui/remoteEntry.js",
        exposes: [
          { name: "Sidebar", module: "./Sidebar", slot: "sidebar" },
          { name: "Header", module: "./Header", slot: "header" },
        ],
      },
      {
        singletons: { react: { version: "18.0.0" } },
        init(h) {
          h.register({ components: { Sidebar: ComponentA, Header: () => null } });
        },
      },
    );
    await mgr.registerExtensionRemote(
      "com.acme.b",
      {
        entry: "ui/remoteEntry.js",
        exposes: [{ name: "Sidebar", module: "./Sidebar", slot: "sidebar" }],
      },
      {
        singletons: { react: { version: "18.0.0" } },
        init(h) {
          h.register({ components: { Sidebar: ComponentB } });
        },
      },
    );

    const sidebar = mgr.resolveSlot("sidebar");
    expect(sidebar.map((r) => r.extensionId)).toEqual([
      "com.acme.a",
      "com.acme.b",
    ]);
    expect(sidebar[0]?.component).toBe(ComponentA);
    expect(sidebar[1]?.component).toBe(ComponentB);

    const header = mgr.resolveSlot("header");
    expect(header).toHaveLength(1);
    expect(header[0]?.extensionId).toBe("com.acme.a");

    expect(mgr.resolveSlot("nowhere")).toEqual([]);
  });
});
