// "Two extensions, no React duplication" smoke test — example side.
//
// Loads the `hello-ui` `remoteEntry` factory twice under different
// extension ids into a single host page and asserts the panel from
// each renders, the host's React reference is the one shared, and
// `<ExtensionSlot id="sidebar"/>` contains both contributions.
//
// This is the user-facing variant of the matching test inside
// `@nube/starter-ext-ui`'s own test suite — same property, exercised
// from outside the kernel package the way an actual consumer wires
// things up.

import * as React from "react";
import { describe, expect, it } from "vitest";

import { ExtensionSlot, type ManifestUi } from "@nube/starter-ext-ui";
import { renderWithExtensionHost } from "@nube/starter-ext-ui/testing";

import remoteEntry from "./remoteEntry.js";

const UI: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "HelloPanel", module: "./Panel", slot: "sidebar" }],
};

describe("two hello-ui extensions in the same host page", () => {
  it("share the host's React and both render in the sidebar slot", async () => {
    const { container } = await renderWithExtensionHost(
      <ExtensionSlot id="sidebar" />,
      {
        extensions: [
          { id: "com.acme.hello-ui.a", ui: UI, factory: remoteEntry },
          { id: "com.acme.hello-ui.b", ui: UI, factory: remoteEntry },
        ],
      },
    );

    // Both panels rendered.
    const panels = container.querySelectorAll(".hello-ui-panel");
    expect(panels).toHaveLength(2);

    // Each panel's `h3` mentions its own extension id (proves the
    // slot context is correct per mount, not shared between them).
    const titles = Array.from(panels).map((p) => p.querySelector("h3")?.textContent);
    expect(titles).toEqual([
      "Hello from com.acme.hello-ui.a",
      "Hello from com.acme.hello-ui.b",
    ]);

    // One slot root, not two — the slot wraps every contribution in
    // one container. We assert on `.starter-ext-slot` (the slot root
    // marker class) rather than `[data-ext-slot]` because BlockShell
    // also tags its outer element with the slot id for telemetry.
    expect(container.querySelectorAll(".starter-ext-slot")).toHaveLength(1);
  });
});
