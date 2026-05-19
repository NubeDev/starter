// Bootstrap glue between the notes shell and the
// `@nube/starter-ext-ui` Module-Federation runtime.
//
// Responsibilities, in order:
//
// 1. Construct one `ExtensionHostManager` per app lifetime.
//    Singletons: React + react-dom (the four-key SCOPE R11
//    baseline minus query/zustand, since neither is wired into the
//    notes shell). The host's live React instance is what every
//    extension's `remoteEntry.init` binds to via
//    `handle.singletons.react`.
//
// 2. Fetch `GET /extensions` to discover loaded extensions, then for
//    each one with a `contributes.ui` block, dynamic-import its
//    `/extensions/<id>/ui/<entry>` URL and feed the resulting factory
//    to `manager.registerExtensionRemote`. The admin slice serves
//    those static files; the federation runtime negotiates singletons
//    and calls the factory's `init` with a handle bound to the host's
//    React.
//
// 3. Failures per extension are isolated: a bad bundle or a singleton
//    mismatch logs a warning and the next extension still loads. The
//    shell renders whatever did register. This matches SCOPE.md R9 —
//    each extension is its own subtree.
//
// 4. Non-admin tokens see `GET /extensions` return 401/403; this
//    function tolerates that (logs a hint, returns an empty manager)
//    so the notes app still renders for plain users.

import * as React from "react";
import * as ReactDOM from "react-dom";
import { ExtensionHostManager } from "@nube/starter-ext-ui";
import type { StarterClient } from "@nube/starter-client-ts";

export interface BootstrapInput {
  client: StarterClient;
}

export function createExtensionHost(input: BootstrapInput): ExtensionHostManager {
  return new ExtensionHostManager({
    client: input.client,
    singletons: {
      react: { version: React.version, instance: React },
      "react-dom": { version: ReactDOM.version, instance: ReactDOM },
    },
  });
}

/**
 * Discover every extension the server has loaded and register their
 * UI remotes with the manager. Resolves once every reachable bundle
 * has either registered or logged its own failure — never throws.
 */
export async function loadExtensionRemotes(
  host: ExtensionHostManager,
): Promise<void> {
  const client = host.client;
  const inventoryRes = await client.fetch(`${client.baseUrl}/extensions`, {
    headers: client.headers,
  });
  if (!inventoryRes.ok) {
    // Non-admin tokens get 401/403. The notes app still works; the
    // sidebar slot simply renders nothing.
    return;
  }
  const inventory = (await inventoryRes.json()) as Array<{ id: string }>;
  await Promise.all(
    inventory.map(async (row) => {
      try {
        await registerOne(host, row.id);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.warn(`[notes] extension ${row.id} did not register:`, err);
      }
    }),
  );
}

async function registerOne(
  host: ExtensionHostManager,
  id: string,
): Promise<void> {
  const client = host.client;
  const detailRes = await client.fetch(
    `${client.baseUrl}/extensions/${encodeURIComponent(id)}`,
    { headers: client.headers },
  );
  if (!detailRes.ok) return;
  const detail = (await detailRes.json()) as {
    manifest?: {
      contributes?: {
        ui?: {
          entry: string;
          exposes: Array<{ name: string; module: string; slot: string }>;
        };
      };
    };
  };
  const ui = detail.manifest?.contributes?.ui;
  if (!ui) return;

  // `vite-ignore` keeps the dynamic-import bare specifier from being
  // statically analysed. The URL is server-side data, not a literal
  // module path the bundler should pre-resolve.
  const url = `${client.baseUrl}/extensions/${encodeURIComponent(id)}/ui/${ui.entry}`;
  const mod: { default: import("@nube/starter-ext-ui").ExtensionRemoteFactory } =
    await import(/* @vite-ignore */ url);
  await host.registerExtensionRemote(id, ui, mod.default);
}
