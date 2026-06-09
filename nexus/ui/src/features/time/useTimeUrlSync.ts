import { useEffect, useRef } from "react";
import { useSearchParams } from "react-router-dom";

import { useTimeStore } from "@/store/time";
import { parseTimeParams, writeTimeParams } from "@/features/time/urlState";

// Two-way bind the time-range + refresh to the URL query string so a
// dashboard link is shareable and survives a reload (C3). On mount it
// restores any `?from=&to=&refresh=` into the store; thereafter it writes
// store changes back into the URL (replacing history, not pushing — a time
// tweak shouldn't spam the back button).
//
// A mount-once guard separates "restore from URL" from "write to URL" so the
// initial restore doesn't immediately rewrite the same params.
export function useTimeUrlSync(): void {
  const [searchParams, setSearchParams] = useSearchParams();
  const range = useTimeStore((s) => s.range);
  const refresh = useTimeStore((s) => s.refresh);
  const setRange = useTimeStore((s) => s.setRange);
  const setRefresh = useTimeStore((s) => s.setRefresh);
  const restored = useRef(false);

  // Restore once on mount: URL is authoritative for a freshly opened link.
  useEffect(() => {
    if (restored.current) return;
    restored.current = true;
    const parsed = parseTimeParams(searchParams);
    if (parsed.range) setRange(parsed.range);
    if (parsed.refresh !== undefined) setRefresh(parsed.refresh);
    // Intentionally mount-only: subsequent URL edits come from this hook's
    // own writes, which we don't want to re-import as state changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Write store -> URL after restore. Only touch the params if they'd change,
  // to avoid a redundant history replace on every tick.
  useEffect(() => {
    if (!restored.current) return;
    const next = writeTimeParams(searchParams, { range, refresh });
    if (next.toString() !== searchParams.toString()) {
      setSearchParams(next, { replace: true });
    }
  }, [range, refresh, searchParams, setSearchParams]);
}
