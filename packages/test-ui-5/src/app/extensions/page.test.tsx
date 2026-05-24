// Smoke test for the extensions host page.
//
// We pre-build an `ExtensionHostManager` with one registered remote
// that contributes a component to the `main` slot, then assert the
// page mounts it. This proves the wiring of `ExtensionHostProvider`
// + `ExtensionSlot` without depending on a live rubix-agent or a
// federation bundle on disk.

import * as React from "react";
import * as ReactDOM from "react-dom";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";
import {
  ExtensionHostManager,
  type ExtensionRemoteFactory,
  type ManifestUi,
} from "@nube/starter-ext-ui";

import ExtensionsPage from "./page.js";

const MAIN_UI: ManifestUi = {
  entry: "ui/remoteEntry.js",
  exposes: [{ name: "Main", module: "./main", slot: "main" }],
};

describe("ExtensionsPage", () => {
  it("renders ExtensionSlot id=main and mounts a registered remote", async () => {
    const factory: ExtensionRemoteFactory = {
      singletons: { react: { version: React.version } },
      init(handle) {
        const Host = handle.singletons["react"] as typeof React;
        const Panel = () =>
          Host.createElement("div", { "data-ext": "example" }, "hello-from-com.rubix.example");
        handle.register({ components: { Main: Panel } });
      },
    };

    const host = new ExtensionHostManager({
      client: new StarterClient({ baseUrl: "http://localhost.invalid" }),
      singletons: {
        react: { version: React.version, instance: React },
        "react-dom": { version: ReactDOM.version, instance: ReactDOM },
      },
    });
    await host.registerExtensionRemote("com.rubix.example", MAIN_UI, factory);

    const { container } = render(<ExtensionsPage host={host} />);

    // The slot root mounted.
    expect(container.querySelector("[data-ext-slot='main']")).not.toBeNull();
    // The extension's panel is visible inside it.
    expect(screen.getByText("hello-from-com.rubix.example")).toBeTruthy();
  });

  it("renders an empty slot region when no remotes are registered", () => {
    const host = new ExtensionHostManager({
      client: new StarterClient({ baseUrl: "http://localhost.invalid" }),
      singletons: {
        react: { version: React.version, instance: React },
        "react-dom": { version: ReactDOM.version, instance: ReactDOM },
      },
    });
    const { container } = render(<ExtensionsPage host={host} />);
    const slot = container.querySelector("[data-ext-slot='main']");
    expect(slot).not.toBeNull();
    expect(slot?.children.length).toBe(0);
  });
});
