// `<ThemeProvider>` — wraps the app, manages preference + resolved
// theme, listens to `prefers-color-scheme` for `system` mode.

import { createContext, useEffect, useMemo, useState, type ReactNode } from "react";

import type { Theme, ThemeContextValue } from "./types.js";

export const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = "starter:theme";

export interface ThemeProviderProps {
  /** Default theme if nothing is in `localStorage`. */
  defaultTheme?: Theme;
  children: ReactNode;
}

export function ThemeProvider({ defaultTheme = "system", children }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(() => readStored() ?? defaultTheme);
  const [resolved, setResolved] = useState<"light" | "dark">(() => resolveTheme(theme));

  useEffect(() => {
    setResolved(resolveTheme(theme));
    if (theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => setResolved(mq.matches ? "dark" : "light");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", resolved === "dark");
  }, [resolved]);

  const value = useMemo<ThemeContextValue>(
    () => ({
      theme,
      resolved,
      setTheme: (next) => {
        localStorage.setItem(STORAGE_KEY, next);
        setThemeState(next);
      },
    }),
    [theme, resolved],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

function readStored(): Theme | null {
  if (typeof window === "undefined") return null;
  const raw = localStorage.getItem(STORAGE_KEY);
  return raw === "light" || raw === "dark" || raw === "system" ? raw : null;
}

function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme !== "system") return theme;
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
