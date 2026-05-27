// `bootstrapExtensions` — walk the host's `GET /extensions` inventory
// and `registerExtensionRemote` every enabled entry with a UI
// contribution.
//
// Without this helper every host shell has to write the same loop:
// fetch the list, fetch each manifest, dynamic-import its
// `contributes.ui.entry`, then call
// `manager.registerExtensionRemote`. Keeping the loop here means
// every consumer of `@nube/starter-ext-ui` gets it for free, and
// fixes the typical pitfalls in one place (URL building for
// `entry` paths, lifecycle-state filtering, robust per-extension
// error isolation).
//
// The helper is intentionally framework-light: no React, no
// `useEffect`, no query-cache. Hosts call it once at boot (or after
// `refresh()` on `useExtensionHost`) and the manager fires its
// usual change notifications so `<ExtensionSlot/>` re-renders.

import type { ExtensionHostManager, ExtensionRemoteFactory } from "./host-manager.js";

/** Per-extension JSON shape returned by `GET /extensions/<id>`.
 * Mirrors the relevant slice of `starter_ext_server::routes::ExtensionDetail`
 * — only the fields this loop reads are declared; extra fields are
 * tolerated. */
export interface BootstrapExtensionDetail {
  id: string;
  /** `enabled` | `disabled` per `EnablementState`. */
  enabled: string;
  /** `validated` | `failed` | runtime state. */
  state: string;
  manifest: {
    id?: string;
    version?: string;
    contributes?: {
      ui?: {
        entry: string;
        exposes?: ReadonlyArray<{ name: string; module: string; slot: string }>;
      };
    };
  } | null;
}

/** Per-extension JSON shape returned by `GET /extensions`. */
export interface BootstrapExtensionSummary {
  id: string;
  /** Lifecycle state on the server: `running`, `stopped`, ... */
  state?: string;
  /** Persisted enablement: `enabled` | `disabled`. */
  enabled?: boolean;
}

export interface BootstrapOptions {
  /**
   * Base path on the host's `StarterClient.baseUrl` that hosts the
   * extension admin routes. Defaults to `/extensions` (the
   * starter-default mount); rubix-style consumers that mount under
   * a versioned API root pass `/api/v1/extensions`.
   */
  basePath?: string;
  /**
   * Override the dynamic import. Real callers leave this unset
   * (defaults to `(url) => import(/* @vite-ignore *\/ url)`); tests
   * inject a stub.
   */
  importRemote?: (url: string) => Promise<unknown>;
  /**
   * Called once per extension after `registerExtensionRemote`
   * resolves. Defaults to a no-op. Useful for surfacing each
   * registration outcome to the operator (a toast, a log line).
   */
  onRegistered?: (id: string) => void;
  /**
   * Called once per extension whose registration failed. Defaults
   * to `console.warn`. Per-extension failures are isolated — one
   * bad remote does not abort the loop.
   */
  onError?: (id: string, error: unknown) => void;
}

/** Outcome counts returned from [`bootstrapExtensions`]. */
export interface BootstrapResult {
  /** Extensions seen in the list response. */
  seen: number;
  /** Extensions skipped because they declare no UI contribution. */
  skippedNoUi: number;
  /** Extensions skipped because the server reports them as not
   * enabled (operator disabled the row). */
  skippedDisabled: number;
  /** Extensions successfully registered. */
  registered: number;
  /** Extensions whose registration threw — see `onError` for detail. */
  failed: number;
}

/**
 * Bootstrap every enabled, UI-contributing extension on
 * `manager.client`. Idempotent: calling twice replaces prior
 * registrations (the manager's own contract).
 *
 * Returns a counts summary so the host shell can render
 * "X registered, Y failed" without subscribing to the manager.
 */
export async function bootstrapExtensions(
  manager: ExtensionHostManager,
  options: BootstrapOptions = {},
): Promise<BootstrapResult> {
  const basePath = (options.basePath ?? "/extensions").replace(/\/$/, "");
  const importRemote =
    options.importRemote ?? ((url: string) => import(/* @vite-ignore */ url));
  const onError =
    options.onError ??
    ((id, err) => {
      // eslint-disable-next-line no-console
      console.warn(`[starter-ext-ui] bootstrap registration failed for ${id}:`, err);
    });
  const onRegistered = options.onRegistered;

  const client = manager.client;
  const listUrl = `${client.baseUrl}${basePath}`;
  const listRes = await client.fetch(listUrl, { headers: client.headers });
  if (!listRes.ok) {
    throw new Error(
      `bootstrapExtensions: GET ${basePath} failed: ${listRes.status}`,
    );
  }
  // The server returns either a bare array or `{ extensions: [...] }`
  // depending on consumer. Accept both.
  const listJson = (await listRes.json()) as
    | ReadonlyArray<BootstrapExtensionSummary>
    | { extensions: ReadonlyArray<BootstrapExtensionSummary> };
  const summaries: ReadonlyArray<BootstrapExtensionSummary> = Array.isArray(
    listJson,
  )
    ? (listJson as ReadonlyArray<BootstrapExtensionSummary>)
    : (listJson as { extensions: ReadonlyArray<BootstrapExtensionSummary> })
        .extensions;

  const result: BootstrapResult = {
    seen: summaries.length,
    skippedNoUi: 0,
    skippedDisabled: 0,
    registered: 0,
    failed: 0,
  };

  for (const sum of summaries) {
    // Treat absence of `enabled` as enabled (the server may omit it
    // for in-memory or test inventories that have no PG store).
    if (sum.enabled === false) {
      result.skippedDisabled += 1;
      continue;
    }
    try {
      const detailUrl = `${client.baseUrl}${basePath}/${encodeURIComponent(sum.id)}`;
      const detailRes = await client.fetch(detailUrl, {
        headers: client.headers,
      });
      if (!detailRes.ok) {
        throw new Error(`GET ${basePath}/${sum.id} failed: ${detailRes.status}`);
      }
      const detail = (await detailRes.json()) as BootstrapExtensionDetail;
      const ui = detail.manifest?.contributes?.ui;
      if (!ui || !ui.entry) {
        result.skippedNoUi += 1;
        continue;
      }
      // Build the remoteEntry URL relative to the bundle's `ui/` mount.
      // The starter-ext-server route is `${basePath}/<id>/ui/*path` where
      // `*path` is resolved against `<bundle_dir>/<dirname(entry)>`. So
      // strip a leading `ui/` from the manifest entry to avoid building
      // `/extensions/<id>/ui/ui/remoteEntry.js`.
      const entrySuffix = ui.entry
        .replace(/^\/+/, "")
        .replace(/^ui\//, "");
      const entryUrl = `${client.baseUrl}${basePath}/${encodeURIComponent(
        sum.id,
      )}/ui/${entrySuffix}`;
      const mod = (await importRemote(entryUrl)) as
        | ExtensionRemoteFactory
        | { default: ExtensionRemoteFactory };
      const factory: ExtensionRemoteFactory =
        "init" in mod ? (mod as ExtensionRemoteFactory) : mod.default;
      if (!factory || typeof factory.init !== "function") {
        throw new Error(
          `remoteEntry at ${entryUrl} did not export an ExtensionRemoteFactory`,
        );
      }
      await manager.registerExtensionRemote(
        sum.id,
        { entry: ui.entry, exposes: ui.exposes ?? [] },
        factory,
      );
      result.registered += 1;
      if (onRegistered) onRegistered(sum.id);
    } catch (err) {
      result.failed += 1;
      onError(sum.id, err);
    }
  }
  return result;
}
