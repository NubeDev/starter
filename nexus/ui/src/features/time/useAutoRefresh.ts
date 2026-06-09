import { useEffect } from "react";

import { useTimeStore } from "@/store/time";

// Drives the auto-refresh loop: when an interval is set, bump the time
// store's `tick` every `refresh` seconds so relative ranges re-resolve and
// the panel queries re-run. Pauses while the tab is hidden (no point
// hammering the DB for a dashboard nobody is looking at) and fires once
// immediately on becoming visible again so a returning user sees fresh data.
//
// The timer lives here, not in the store, because it's a side effect tied to
// a mounted dashboard view; unmounting the dashboard tears it down.
export function useAutoRefresh(): void {
  const refresh = useTimeStore((s) => s.refresh);
  const bump = useTimeStore((s) => s.bump);

  useEffect(() => {
    if (refresh <= 0) return;

    let timer: ReturnType<typeof setInterval> | undefined;

    const start = () => {
      stop();
      timer = setInterval(() => {
        // Guard inside the tick too: a tab hidden between visibility events
        // (e.g. minimised) should not refetch.
        if (document.visibilityState === "visible") bump();
      }, refresh * 1000);
    };
    const stop = () => {
      if (timer) clearInterval(timer);
      timer = undefined;
    };

    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        bump();
        start();
      } else {
        stop();
      }
    };

    if (document.visibilityState === "visible") start();
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      stop();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [refresh, bump]);
}
