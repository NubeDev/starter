// End-to-end "two extensions, no React duplication" smoke test.
//
// This is the user-visible version of the assertion already made in
// `host-manager.test.ts` (which exercises the negotiator in
// isolation). Here we render two extension panels inside an
// `<ExtensionSlot/>` and check:
//
// 1. Both panels mount (no singleton-mismatch error).
// 2. The host's React reference is what each panel's `init` saw.
// 3. The panels' content lands in the DOM under the slot root, in
//    the order the host registered them.

import * as React from "react";
import { describe, expect, it } from "vitest";

import { renderWithExtensionHost } from "./testing/render.js";
import { ExtensionSlot } from "./extension-slot.js";
import type { ExtensionRemoteFactory, ManifestUi } from "./host-manager.js";

const UI_A: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "Panel", module: "./Panel", slot: "sidebar" }],
};
const UI_B: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "Panel", module: "./Panel", slot: "sidebar" }],
};

describe("two-extensions-no-React-duplication (slot integration)", () => {
  it("mounts two extension panels through one host React instance", async () => {
    const seen: { id: string; react: unknown }[] = [];

    const factoryA: ExtensionRemoteFactory = {
      singletons: { react: { version: React.version } },
      init(h) {
        const Host = h.singletons.react as typeof React;
        const PanelA = () => Host.createElement("div", { "data-ext": "a" }, "A");
        seen.push({ id: h.id, react: h.singletons.react });
        h.register({ components: { Panel: PanelA } });
      },
    };

    const factoryB: ExtensionRemoteFactory = {
      singletons: { react: { version: React.version } },
      init(h) {
        const Host = h.singletons.react as typeof React;
        const PanelB = () => Host.createElement("div", { "data-ext": "b" }, "B");
        seen.push({ id: h.id, react: h.singletons.react });
        h.register({ components: { Panel: PanelB } });
      },
    };

    const { container } = await renderWithExtensionHost(
      <ExtensionSlot id="sidebar" />,
      {
        extensions: [
          { id: "com.acme.a", ui: UI_A, factory: factoryA },
          { id: "com.acme.b", ui: UI_B, factory: factoryB },
        ],
      },
    );

    // Both inits ran with the same React reference (the host's).
    expect(seen).toHaveLength(2);
    expect(seen[0]?.react).toBe(React);
    expect(seen[1]?.react).toBe(React);

    // Both panels rendered into the DOM under the slot.
    const a = container.querySelector("[data-ext='a']");
    const b = container.querySelector("[data-ext='b']");
    expect(a?.textContent).toBe("A");
    expect(b?.textContent).toBe("B");

    // And there is *one* slot root, not two.
    const slotRoots = container.querySelectorAll("[data-ext-slot='sidebar']");
    expect(slotRoots).toHaveLength(1);
  });
});
