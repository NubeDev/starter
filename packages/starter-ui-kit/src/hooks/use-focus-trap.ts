// `useFocusTrap()` — keep keyboard focus inside a container while
// it's mounted (used by dialog primitives).

import { useEffect, type RefObject } from "react";

export function useFocusTrap(containerRef: RefObject<HTMLElement>): void {
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // TODO(ap): full focus-trap impl. Stub preserves the hook
    // signature so consumers can call it today.
  }, [containerRef]);
}
