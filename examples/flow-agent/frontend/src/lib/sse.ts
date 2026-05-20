// Tiny EventSource wrapper with auto-reconnect and typed JSON parse.

import { useEffect, useRef } from "react";

export function useSse<T>(url: string | null, onEvent: (ev: T) => void) {
  const handlerRef = useRef(onEvent);
  handlerRef.current = onEvent;

  useEffect(() => {
    if (!url) return;
    const es = new EventSource(url);
    es.onmessage = (e) => {
      try {
        handlerRef.current(JSON.parse(e.data) as T);
      } catch {
        /* malformed event — drop */
      }
    };
    es.onerror = () => {
      // EventSource auto-reconnects on close. We rely on that.
    };
    return () => es.close();
  }, [url]);
}
