// Internal — `renderWithExtensionHost` implementation.

import * as React from "react";
import * as ReactDOM from "react-dom";
import { render, type RenderResult } from "@testing-library/react";

import { StarterClient } from "@nube/starter-client-ts";

import { ExtensionHostManager, type ExtensionRemoteFactory, type ManifestUi } from "../host-manager.js";
import { ExtensionHostProvider } from "../host-provider.js";

export interface RenderWithExtensionHostOptions {
  /**
   * Pre-registered remotes. Each entry is registered against the
   * manager before the tree renders.
   */
  extensions?: Array<{
    id: string;
    ui: ManifestUi;
    factory: ExtensionRemoteFactory;
  }>;
  /**
   * Override the default `StarterClient`. The default is a stub
   * pointing at `http://localhost.invalid` — tests that exercise the
   * client should pass an `msw`-wired instance.
   */
  client?: StarterClient;
  /**
   * Override the host's provided singletons. By default the harness
   * advertises `react` + `react-dom` at the versions of the modules
   * loaded into the test process. Override to drive
   * singleton-mismatch tests.
   */
  singletons?: Record<string, { version: string; instance: unknown }>;
}

/**
 * Render a tree wrapped in `ExtensionHostProvider` with optional
 * pre-registered remotes. Returns the manager alongside the standard
 * `RenderResult` so tests can assert on registration state.
 */
export async function renderWithExtensionHost(
  ui: React.ReactElement,
  options: RenderWithExtensionHostOptions = {},
): Promise<RenderResult & { manager: ExtensionHostManager }> {
  const client =
    options.client ?? new StarterClient({ baseUrl: "http://localhost.invalid" });

  const defaultSingletons = {
    react: { version: React.version, instance: React },
    "react-dom": { version: ReactDOM.version, instance: ReactDOM },
  };

  const manager = new ExtensionHostManager({
    client,
    singletons: options.singletons ?? defaultSingletons,
  });

  for (const ext of options.extensions ?? []) {
    await manager.registerExtensionRemote(ext.id, ext.ui, ext.factory);
  }

  const result = render(
    <ExtensionHostProvider host={manager}>{ui}</ExtensionHostProvider>,
  );
  return Object.assign(result, { manager });
}
