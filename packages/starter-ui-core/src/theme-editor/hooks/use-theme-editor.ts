// `useThemeEditor` — owns the document lifecycle.
//
// Mounts the editor against a `ThemeTransport`, hydrates the store on
// first render, exposes a `save()` callback that batches token + shell
// + pending-asset writes into one logical commit, and tears down on
// unmount.
//
// The hook is deliberately single-instance: putting two
// `<ThemeEditorPage>`s in the same tree would race on the shared
// Zustand store. The component-level guard is left to the consumer.

import { useCallback, useEffect, useRef, useState } from "react";

import { useThemeEditorStore } from "../store.js";
import type { ThemeTransport } from "../transport.js";
import type { ThemeDocument } from "../types.js";

export interface UseThemeEditorResult {
  /** `true` between mount and first successful load. */
  isLoading: boolean;
  /** Set if the initial load (or a subsequent save) threw. The error
   * message is forwarded so the page can toast it. */
  error: Error | null;
  /** Server-recorded asset URLs from the most recent load/save. */
  logoUrl: string | null;
  faviconUrl: string | null;
  /** Persist the current store state. Resolves once the document and
   * any pending assets are committed. */
  save: () => Promise<void>;
  /** Re-fetch from the transport, discarding local edits. */
  reload: () => Promise<void>;
}

export function useThemeEditor(transport: ThemeTransport): UseThemeEditorResult {
  const hydrate = useThemeEditorStore((s) => s.hydrate);
  const markSaved = useThemeEditorStore((s) => s.markSaved);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [logoUrl, setLogoUrl] = useState<string | null>(null);
  const [faviconUrl, setFaviconUrl] = useState<string | null>(null);

  // Guard against double-mount under React 18 strict mode and against
  // transport-identity changes that would otherwise wipe an in-progress
  // edit. The editor loads exactly once per page mount.
  const loadedRef = useRef(false);

  const applyDocument = useCallback(
    (doc: ThemeDocument) => {
      hydrate(doc.theme_styles, doc.shell);
      setLogoUrl(doc.logo_url ?? null);
      setFaviconUrl(doc.favicon_url ?? null);
    },
    [hydrate],
  );

  const reload = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const doc = await transport.load();
      applyDocument(doc);
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setIsLoading(false);
    }
  }, [transport, applyDocument]);

  useEffect(() => {
    if (loadedRef.current) return;
    loadedRef.current = true;
    void reload();
  }, [reload]);

  const save = useCallback(async () => {
    setError(null);
    const state = useThemeEditorStore.getState();
    try {
      // Document first — if the asset uploads land but the doc save
      // throws, the consumer still ends up with stale token state.
      // Doing the doc first means a partial failure leaves consistent
      // metadata pointing at the *old* assets.
      const doc = await transport.save({
        theme_styles: state.styles,
        shell: state.shell,
      });

      // Assets only if the user touched them. `null` = upload was
      // cleared by the user (delete on server); `undefined` = no
      // pending change.
      if (state.pendingLogo !== null) {
        await transport.setLogo(state.pendingLogo ?? null);
      }
      if (state.pendingFavicon !== null) {
        await transport.setFavicon(state.pendingFavicon ?? null);
      }

      setLogoUrl(doc.logo_url ?? null);
      setFaviconUrl(doc.favicon_url ?? null);
      markSaved();
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      setError(err);
      throw err;
    }
  }, [transport, markSaved]);

  return { isLoading, error, logoUrl, faviconUrl, save, reload };
}
