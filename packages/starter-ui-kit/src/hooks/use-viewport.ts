// `useViewport()` — observe the window's inner dimensions. Visual
// concern only.

import { useEffect, useState } from "react";

export interface Viewport {
  width: number;
  height: number;
}

export function useViewport(): Viewport {
  const [vp, setVp] = useState<Viewport>(() => current());

  useEffect(() => {
    const onResize = () => setVp(current());
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return vp;
}

function current(): Viewport {
  if (typeof window === "undefined") return { width: 0, height: 0 };
  return { width: window.innerWidth, height: window.innerHeight };
}
