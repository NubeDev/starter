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
import {
  ExtensionHostManager,
  SINGLETON_REACT,
  SINGLETON_REACT_DOM,
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
  type ExtensionHostTelemetryEvent,
  type ExtensionHostTelemetrySink,
} from "@nube/starter-ext-ui";
import type { StarterClient } from "@nube/starter-client-ts";
import { PreferencesContext } from "@nube/starter-ui-core/preferences";
import {
  IntlContext,
  registerExtensionMessages,
  unregisterExtensionMessages,
  type LanguageTag,
} from "@nube/starter-ui-core/i18n";

/**
 * The semver string the host declares for each ui-core singleton.
 * Bumping the major refuses to load extensions built against the old
 * major (D-NP.10). Bumping the minor passes the load check but emits
 * `extension.singleton_minor_drift` for any extension still on an
 * older minor. Patch drift is silent.
 *
 * Pinned here (not read from `package.json#version`) because the
 * singleton contract — `ResolvedPreferences` shape + hook surface +
 * IntlShape consumers — is what extensions key off, not the ui-core
 * release number. Moving the package to `1.0.0` is independent.
 */
export const UI_CORE_PREFERENCES_VERSION = "1.0.0";
export const UI_CORE_I18N_VERSION = "1.0.0";

export interface BootstrapInput {
  client: StarterClient;
  /**
   * Optional telemetry sink for the host. When omitted, the host
   * logs `extension.singleton_mismatch` as a console error and
   * `extension.singleton_minor_drift` as a console warn. Production
   * deployments pass a sink that forwards to
   * `starter-observability`.
   */
  telemetry?: ExtensionHostTelemetrySink;
}

/** Default sink — console output. Production wires a real sink. */
function consoleTelemetry(event: ExtensionHostTelemetryEvent): void {
  if (event.kind === "extension.singleton_mismatch") {
    // eslint-disable-next-line no-console
    console.error(
      `[notes] extension ${event.extensionId} refused: singleton-mismatch`,
      event.reasons.map((r) => r.reason),
    );
    return;
  }
  // eslint-disable-next-line no-console
  console.warn(
    `[notes] extension ${event.extensionId} loaded with minor drift`,
    event.drifts.map(
      (d) => `${d.pkg}: host ${d.hostVersion}, extension ${d.extensionVersion}`,
    ),
  );
}

export function createExtensionHost(input: BootstrapInput): ExtensionHostManager {
  return new ExtensionHostManager({
    client: input.client,
    telemetry: input.telemetry ?? consoleTelemetry,
    singletons: {
      [SINGLETON_REACT]: { version: React.version, instance: React },
      [SINGLETON_REACT_DOM]: { version: ReactDOM.version, instance: ReactDOM },
      // The two new ui-core singletons. The "instance" is the React
      // Context object itself; Stage-3 SDK hooks call
      // `useContext(handle.singletons["@nube/starter-ui-core/preferences"])`
      // against the host's instance instead of the extension's own
      // bundled copy — one source of truth across the federation
      // boundary (D-NP.1, examples/notes/user-pref.md § Stage 2).
      [SINGLETON_UI_CORE_PREFERENCES]: {
        version: UI_CORE_PREFERENCES_VERSION,
        instance: PreferencesContext,
      },
      [SINGLETON_UI_CORE_I18N]: {
        version: UI_CORE_I18N_VERSION,
        instance: IntlContext,
      },
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

/**
 * Per-extension Stage-5 i18n manifest. Populated by `registerOne`
 * from `manifest.contributes.i18n.catalogs` and consumed by
 * [`useExtensionCatalogLoader`] to lazy-fetch the active language's
 * catalog and merge it through `registerExtensionMessages`.
 *
 * Module-level (not React state) because the host-manager itself
 * doesn't retain the manifest blocks — keeping a small parallel
 * registry avoids a wider refactor.
 */
interface ExtensionCatalogManifest {
  /** Language tag → bundle-relative path. We only use the keys
   * client-side (to decide which language tags the extension
   * supports); the path is informational. */
  catalogs: Record<string, string>;
}

const EXTENSION_CATALOGS = new Map<string, ExtensionCatalogManifest>();

/** Test-/dev-only — read the set of extensions that registered an
 * i18n manifest. The notes app uses it indirectly via
 * `useExtensionCatalogLoader`; exposed so unit tests can assert the
 * discovery step ran. */
export function _listExtensionCatalogsForTesting(): ReadonlyMap<string, ExtensionCatalogManifest> {
  return EXTENSION_CATALOGS;
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
        i18n?: {
          catalogs?: Record<string, string>;
        };
      };
    };
  };
  // Remember the i18n manifest before the UI gate below — an
  // extension may ship catalogs without exposing a panel (server-only
  // strings that show up in admin chrome).
  const catalogs = detail.manifest?.contributes?.i18n?.catalogs;
  if (catalogs && Object.keys(catalogs).length > 0) {
    EXTENSION_CATALOGS.set(id, { catalogs });
  } else {
    EXTENSION_CATALOGS.delete(id);
    unregisterExtensionMessages(id);
  }
  const ui = detail.manifest?.contributes?.ui;
  if (!ui) return;

  // `vite-ignore` keeps the dynamic-import bare specifier from being
  // statically analysed. The URL is server-side data, not a literal
  // module path the bundler should pre-resolve.
  const url = `${client.baseUrl}/extensions/${encodeURIComponent(id)}/${ui.entry}`;
  const mod: { default: import("@nube/starter-ext-ui").ExtensionRemoteFactory } =
    await import(/* @vite-ignore */ url);
  await host.registerExtensionRemote(id, ui, mod.default);
}
