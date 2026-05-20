import { useEffect, useRef } from "react";

export function useAutosizeTextarea(
  value: string,
  maxRows = 8,
): React.RefObject<HTMLTextAreaElement | null> {
  const ref = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    const lineHeight = parseFloat(getComputedStyle(el).lineHeight || "20");
    const maxH = lineHeight * maxRows;
    el.style.height = `${Math.min(el.scrollHeight, maxH)}px`;
    el.style.overflowY = el.scrollHeight > maxH ? "auto" : "hidden";
  }, [value, maxRows]);
  return ref;
}
