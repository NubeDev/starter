import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { DirectionProvider as RdxDirectionProvider } from "@radix-ui/react-direction";
import { getCookie, removeCookie, setCookie } from "./cookies.js";

// Wraps Radix's DirectionProvider so popovers/dropdowns flip correctly,
// and also mirrors the value onto `<html dir>` so plain CSS (`[dir="rtl"] …`)
// works without a JS bridge. Persisted to a cookie so SSR can read it.

export type Direction = "ltr" | "rtl";

const DEFAULT_DIRECTION: Direction = "ltr";
const COOKIE = "dir";

export interface DirectionContextValue {
  defaultDir: Direction;
  dir: Direction;
  setDir: (dir: Direction) => void;
  resetDir: () => void;
}

const DirectionContext = createContext<DirectionContextValue | null>(null);

export interface DirectionProviderProps {
  children: ReactNode;
  defaultDir?: Direction;
}

export function DirectionProvider({
  children,
  defaultDir = DEFAULT_DIRECTION,
}: DirectionProviderProps) {
  const [dir, _setDir] = useState<Direction>(
    () => (getCookie(COOKIE) as Direction) || defaultDir,
  );

  useEffect(() => {
    document.documentElement.setAttribute("dir", dir);
  }, [dir]);

  const setDir = (d: Direction) => {
    _setDir(d);
    setCookie(COOKIE, d);
  };

  const resetDir = () => {
    _setDir(defaultDir);
    removeCookie(COOKIE);
  };

  return (
    <DirectionContext.Provider value={{ defaultDir, dir, setDir, resetDir }}>
      <RdxDirectionProvider dir={dir}>{children}</RdxDirectionProvider>
    </DirectionContext.Provider>
  );
}

export function useDirection(): DirectionContextValue {
  const ctx = useContext(DirectionContext);
  if (!ctx) throw new Error("useDirection must be used within a DirectionProvider");
  return ctx;
}
