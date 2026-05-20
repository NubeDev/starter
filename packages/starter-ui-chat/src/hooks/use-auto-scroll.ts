import { useCallback, useEffect, useRef, useState } from "react";

export interface UseAutoScrollOptions {
  threshold?: number;
}

export interface UseAutoScrollReturn<T extends HTMLElement> {
  ref: React.RefObject<T | null>;
  isPinned: boolean;
  scrollToBottom: (smooth?: boolean) => void;
}

// Sticks to the bottom while pinned; releases when the user scrolls up.
export function useAutoScroll<T extends HTMLElement = HTMLDivElement>(
  deps: ReadonlyArray<unknown>,
  opts: UseAutoScrollOptions = {},
): UseAutoScrollReturn<T> {
  const { threshold = 64 } = opts;
  const ref = useRef<T | null>(null);
  const [isPinned, setPinned] = useState(true);

  const scrollToBottom = useCallback((smooth = false) => {
    const el = ref.current;
    if (!el) return;
    el.scrollTo({
      top: el.scrollHeight,
      behavior: smooth ? "smooth" : "auto",
    });
  }, []);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = () => {
      const distance = el.scrollHeight - el.scrollTop - el.clientHeight;
      setPinned(distance <= threshold);
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [threshold]);

  useEffect(() => {
    if (isPinned) scrollToBottom();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return { ref, isPinned, scrollToBottom };
}
