// `ExtensionHostManager` unit tests.
//
// Heavy on singleton negotiation because that is the single biggest
// load-bearing rule for the host runtime (SCOPE.md
// §"Singleton-mismatch handling (UI)" — load-time refusal).

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import {
  ExtensionHostManager,
  type ExtensionHostTelemetryEvent,
  type ExtensionRemoteFactory,
  type ManifestUi,
  type SingletonProvision,
} from "./host-manager.js";
import {
  SingletonMismatchError,
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
} from "./singletons.js";

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

describe("singleton telemetry (Stage 2 — prefs + i18n singletons)", () => {
  // The two new ui-core singletons. The "instance" is a sentinel
  // marker — in production it's the React Context object; here it's
  // any unique reference so we can assert the host hands it through
  // unchanged.
  const HOST_PREFS_CTX = { __id: "the-one-PreferencesContext" } as const;
  const HOST_INTL_CTX = { __id: "the-one-IntlContext" } as const;

  function makeWithUiCore(
    extraOpts: { telemetry?: (e: ExtensionHostTelemetryEvent) => void } = {},
  ) {
    return new ExtensionHostManager({
      client: new StarterClient({ baseUrl: "http://localhost.invalid" }),
      singletons: {
        react: { version: "18.3.1", instance: HOST_REACT },
        "react-dom": { version: "18.3.1", instance: HOST_REACT_DOM },
        [SINGLETON_UI_CORE_PREFERENCES]: {
          version: "1.3.0",
          instance: HOST_PREFS_CTX,
        },
        [SINGLETON_UI_CORE_I18N]: {
          version: "1.3.0",
          instance: HOST_INTL_CTX,
        },
      },
      telemetry: extraOpts.telemetry,
    });
  }

  it("hands the host's PreferencesContext + IntlContext to the extension's init", async () => {
    const mgr = makeWithUiCore();
    let prefs: unknown = null;
    let intl: unknown = null;
    await mgr.registerExtensionRemote("com.acme.a", UI, {
      singletons: {
        react: { version: "18.3.1" },
        [SINGLETON_UI_CORE_PREFERENCES]: { version: "1.3.0" },
        [SINGLETON_UI_CORE_I18N]: { version: "1.3.0" },
      },
      init(handle) {
        prefs = handle.singletons[SINGLETON_UI_CORE_PREFERENCES];
        intl = handle.singletons[SINGLETON_UI_CORE_I18N];
      },
    });
    expect(prefs).toBe(HOST_PREFS_CTX);
    expect(intl).toBe(HOST_INTL_CTX);
  });

  it("emits extension.singleton_mismatch on major mismatch and refuses to load", async () => {
    const events: ExtensionHostTelemetryEvent[] = [];
    const mgr = makeWithUiCore({ telemetry: (e) => events.push(e) });

    await expect(
      mgr.registerExtensionRemote("com.acme.bad", UI, {
        singletons: {
          react: { version: "18.3.1" },
          // Built against the next major of ui-core/preferences.
          [SINGLETON_UI_CORE_PREFERENCES]: { version: "2.0.0" },
        },
        init() {
          throw new Error("init must not run on mismatch");
        },
      }),
    ).rejects.toBeInstanceOf(SingletonMismatchError);

    expect(events).toHaveLength(1);
    const ev = events[0]!;
    expect(ev.kind).toBe("extension.singleton_mismatch");
    expect(ev.severity).toBe("error");
    expect(ev.extensionId).toBe("com.acme.bad");
    if (ev.kind === "extension.singleton_mismatch") {
      expect(ev.reasons.map((r) => r.pkg)).toContain(
        SINGLETON_UI_CORE_PREFERENCES,
      );
    }
    expect(mgr.getRemote("com.acme.bad")).toBeUndefined();
  });

  it("emits extension.singleton_minor_drift when the extension is behind on minor but still loads", async () => {
    const events: ExtensionHostTelemetryEvent[] = [];
    const mgr = makeWithUiCore({ telemetry: (e) => events.push(e) });

    await mgr.registerExtensionRemote("com.acme.lag", UI, {
      singletons: {
        react: { version: "18.3.1" },
        // Host is on 1.3.0; extension built against 1.1.0 — minor
        // drift of 2.
        [SINGLETON_UI_CORE_PREFERENCES]: { version: "1.1.0" },
      },
      init() {},
    });

    expect(mgr.getRemote("com.acme.lag")).toBeDefined();
    const drift = events.find(
      (e) => e.kind === "extension.singleton_minor_drift",
    );
    expect(drift).toBeDefined();
    if (drift && drift.kind === "extension.singleton_minor_drift") {
      expect(drift.severity).toBe("warn");
      expect(drift.extensionId).toBe("com.acme.lag");
      expect(drift.drifts).toHaveLength(1);
      const d = drift.drifts[0]!;
      expect(d.pkg).toBe(SINGLETON_UI_CORE_PREFERENCES);
      expect(d.hostVersion).toBe("1.3.0");
      expect(d.extensionVersion).toBe("1.1.0");
      expect(d.driftMinors).toBe(2);
    }
  });

  it("stays silent on patch drift", async () => {
    const events: ExtensionHostTelemetryEvent[] = [];
    const mgr = makeWithUiCore({ telemetry: (e) => events.push(e) });

    await mgr.registerExtensionRemote("com.acme.patch", UI, {
      singletons: {
        react: { version: "18.3.1" },
        // Host 1.3.0 vs extension 1.3.5 — same minor, patch drift only.
        [SINGLETON_UI_CORE_PREFERENCES]: { version: "1.3.5" },
      },
      init() {},
    });

    expect(events).toHaveLength(0);
  });

  it("does not flag the host being behind on minor (only extension-behind drift)", async () => {
    const events: ExtensionHostTelemetryEvent[] = [];
    const mgr = makeWithUiCore({ telemetry: (e) => events.push(e) });

    await mgr.registerExtensionRemote("com.acme.ahead", UI, {
      singletons: {
        react: { version: "18.3.1" },
        // Host 1.3.0; extension built against 1.5.0. Same major;
        // host needs updating, but that is not the extension's fault
        // and the panel will work — no drift event.
        [SINGLETON_UI_CORE_PREFERENCES]: { version: "1.5.0" },
      },
      init() {},
    });

    expect(events).toHaveLength(0);
  });

  it("swallows exceptions from the telemetry sink", async () => {
    const mgr = makeWithUiCore({
      telemetry: () => {
        throw new Error("sink exploded");
      },
    });
    // A mismatch still produces the SingletonMismatchError (the throw
    // is propagated as the host's contract), but the sink throwing
    // does not turn into an unhandled exception.
    await expect(
      mgr.registerExtensionRemote("com.acme.bad", UI, {
        singletons: {
          react: { version: "18.3.1" },
          [SINGLETON_UI_CORE_PREFERENCES]: { version: "2.0.0" },
        },
        init() {},
      }),
    ).rejects.toBeInstanceOf(SingletonMismatchError);
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
