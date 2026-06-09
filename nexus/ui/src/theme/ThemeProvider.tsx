import { useEffect, type ReactNode } from "react";

import { useThemeStore } from "@/theme/store";

// Keeps the theme in sync with the OS when the user's preference is
// "system". The initial paint already happened in `main.tsx` (via
// `initTheme`, before React mounted, so there's no flash); this only
// wires the live `prefers-color-scheme` listener. No DOM writes here —
// all token application flows through the store → `applyTheme`.
export function ThemeProvider({ children }: { children: ReactNode }) {
  const syncSystem = useThemeStore((s) => s.syncSystem);

  useEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    mql.addEventListener("change", syncSystem);
    return () => mql.removeEventListener("change", syncSystem);
  }, [syncSystem]);

  return <>{children}</>;
}
