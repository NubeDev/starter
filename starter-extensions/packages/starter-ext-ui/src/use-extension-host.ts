// `useExtensionHost` — admin-style hook surfacing what the host
// knows about installed extensions.
//
// Two pieces of state:
//
// - The set of *registered remotes* (what `registerExtensionRemote`
//   has run for) — the host's *runtime* state.
// - The set of records from `GET /extensions` — the *server-side*
//   inventory, which is the source of truth for enablement and
//   lifecycle. Reads via the injected `StarterClient` (SCOPE R11 —
//   never raw fetch).
//
// In v0.1 we only join the two views in the hook return type. A
// future iteration may add `enable(id)` / `disable(id)` mutators
// that post to `/extensions/<id>/enable|disable`; the shape is
// reserved so adding them later doesn't break callers.

import * as React from "react";

import { useExtensionHostManager } from "./host-context.js";
import type { RegisteredRemote } from "./host-manager.js";

/**
 * Per-extension view returned by `useExtensionHost()`. Mirrors the
 * `GET /extensions` summary row, with a `registered` flag indicating
 * whether the host shell has actually run `registerExtensionRemote`
 * for it.
 */
export interface ExtensionHostExtensionView {
  id: string;
  version: string | null;
  displayName: string | null;
  state: string;
  registered: boolean;
  /** Present when `registered === true`. */
  remote: RegisteredRemote | null;
}

/** Aggregate view returned by `useExtensionHost()`. */
export interface ExtensionHostView {
  /** Server-side inventory, fetched once on mount; `null` while loading. */
  installed: ReadonlyArray<ExtensionHostExtensionView> | null;
  /** Set of remotes the host has registered locally (live MF state). */
  registered: ReadonlyArray<RegisteredRemote>;
  /** Reload the server-side inventory. */
  refresh(): Promise<void>;
}

/**
 * Hook surfacing the host's extension state. Subscribes to the
 * manager for live registration changes and fetches `/extensions`
 * lazily on mount via `useHostClient()`'s underlying client.
 */
export function useExtensionHost(): ExtensionHostView {
  const mgr = useExtensionHostManager();
  const registered = React.useSyncExternalStore(
    React.useCallback((cb) => mgr.subscribe(cb), [mgr]),
    React.useCallback(() => mgr.listRemotes(), [mgr]),
    React.useCallback(() => mgr.listRemotes(), [mgr]),
  );

  const [installed, setInstalled] = React.useState<
    ReadonlyArray<ExtensionHostExtensionView> | null
  >(null);

  const refresh = React.useCallback(async () => {
    const client = mgr.client;
    // We route through the typed client (SCOPE R11) but the
    // `/extensions` endpoint lives in `starter-ext-server`, not the
    // base `starter-server` — so we use the client's underlying
    // fetch with its configured baseUrl + headers rather than a
    // declaration-merged method.
    const res = await client.fetch(`${client.baseUrl}/extensions`, {
      headers: client.headers,
    });
    if (!res.ok) {
      throw new Error(`GET /extensions failed: ${res.status}`);
    }
    const rows = (await res.json()) as ServerSummary[];
    const localById = new Map(mgr.listRemotes().map((r) => [r.id, r]));
    setInstalled(
      rows.map((row) => ({
        id: row.id,
        version: row.version ?? null,
        displayName: row.display_name ?? null,
        state: row.state,
        registered: localById.has(row.id),
        remote: localById.get(row.id) ?? null,
      })),
    );
  }, [mgr]);

  React.useEffect(() => {
    void refresh().catch((e) => {
      // The hook does not throw — surfacing the error is the
      // consumer's responsibility (they can call refresh() again
      // from a button and catch there).
      // eslint-disable-next-line no-console
      console.warn("[starter-ext-ui] useExtensionHost refresh failed:", e);
    });
  }, [refresh]);

  return { installed, registered, refresh };
}

/**
 * Wire-shape mirror of `starter_ext_server::routes::ExtensionSummary`.
 * Kept locally rather than imported from a codegen module because the
 * extension routes are not in `openapi.json` in v0.1.
 */
interface ServerSummary {
  id: string;
  version?: string;
  display_name?: string;
  state: string;
}
