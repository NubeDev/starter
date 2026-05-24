import * as React from "react";

const MOBILE_BREAKPOINT = 768;
const MOBILE_QUERY = `(max-width: ${MOBILE_BREAKPOINT - 1}px)`;

// Subscribes to `(max-width: 767px)` and returns whether the viewport
// currently matches it. SSR-safe: returns `false` until the client
// hydrates the matchMedia state via useSyncExternalStore.
export function useIsMobile(): boolean {
  return React.useSyncExternalStore(
    (callback) => {
      const mql = window.matchMedia(MOBILE_QUERY);
      mql.addEventListener("change", callback);
      return () => mql.removeEventListener("change", callback);
    },
    () => window.matchMedia(MOBILE_QUERY).matches,
    () => false,
  );
}
