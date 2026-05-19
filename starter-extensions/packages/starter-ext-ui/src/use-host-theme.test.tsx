// `useHostTheme()` round-trip through `<ExtensionSlot>` — proves
// the host's `themeTokens` prop reaches an extension panel and the
// fallback to `getComputedStyle(document.documentElement)` fires
// when the host omits the map.
//
// This is the load-bearing test for the SCOPE.md "extensions
// inherit the host theme via cascade and an opt-in
// `useHostTheme()` hook" guarantee — the kernel never imports the
// host's theme editor brain (`@nube/starter-ui-core/theme-editor`),
// so we exercise the contract without bringing it in.

import * as React from "react";
import { describe, expect, it } from "vitest";

import { useHostTheme } from "@nube/starter-ext-sdk-ts";

import { ExtensionSlot } from "./extension-slot.js";
import { renderWithExtensionHost } from "./testing/render.js";
import type { ExtensionRemoteFactory, ManifestUi } from "./host-manager.js";

const UI: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "Panel", module: "./Panel", slot: "sidebar" }],
};

function makeFactory(): ExtensionRemoteFactory {
  return {
    singletons: { react: { version: React.version } },
    init(h) {
      const Host = h.singletons.react as typeof React;
      const Panel = () => {
        const theme = useHostTheme();
        return Host.createElement(
          "div",
          { "data-ext": "theme-probe" },
          `mode=${theme.mode};primary=${theme.token("primary")};radius=${theme.token("radius")}`,
        );
      };
      h.register({ components: { Panel } });
    },
  };
}

describe("useHostTheme inside <ExtensionSlot>", () => {
  it("reads host-supplied themeTokens map directly", async () => {
    const { container } = await renderWithExtensionHost(
      <ExtensionSlot
        id="sidebar"
        theme="dark"
        themeTokens={{
          primary: "oklch(0.5 0.2 250)",
          radius: "0.625rem",
        }}
      />,
      {
        extensions: [{ id: "com.acme.theme-probe", ui: UI, factory: makeFactory() }],
      },
    );

    const probe = container.querySelector("[data-ext='theme-probe']");
    expect(probe?.textContent).toBe(
      "mode=dark;primary=oklch(0.5 0.2 250);radius=0.625rem",
    );
  });

  it("falls back to getComputedStyle(document.documentElement) when no map is supplied", async () => {
    // Stamp the value the cascade fallback should pick up.
    document.documentElement.style.setProperty("--primary", "oklch(0.7 0.1 30)");
    try {
      const { container } = await renderWithExtensionHost(
        <ExtensionSlot id="sidebar" theme="light" />,
        {
          extensions: [{ id: "com.acme.theme-probe", ui: UI, factory: makeFactory() }],
        },
      );

      const probe = container.querySelector("[data-ext='theme-probe']");
      // `radius` is intentionally absent from the cascade — the hook
      // returns the empty string and the caller decides the default.
      expect(probe?.textContent).toBe(
        "mode=light;primary=oklch(0.7 0.1 30);radius=",
      );
    } finally {
      document.documentElement.style.removeProperty("--primary");
    }
  });
});
