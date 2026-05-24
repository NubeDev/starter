import { createContext, useContext, useState, type ReactNode } from "react";
import { getCookie, setCookie } from "./cookies.js";

// App-shell layout preferences. Lives in `ui-core` (not `ui-kit`) because
// it owns persistence (cookies) and consumer-facing state, which violates
// the kit's R6: zero-I/O rule.
//
// The three axes intentionally have free-form string types parameterised
// on the consumer so an app can extend the set (e.g. a `compact` variant)
// without forking the provider.

export type LayoutMode = "header" | "sidebar";
export type Collapsible = "offcanvas" | "icon" | "none";
export type Variant = "inset" | "sidebar" | "floating";

export interface LayoutContextValue {
  defaultMode: LayoutMode;
  mode: LayoutMode;
  setMode: (m: LayoutMode) => void;
  toggle: () => void;

  defaultCollapsible: Collapsible;
  collapsible: Collapsible;
  setCollapsible: (c: Collapsible) => void;

  defaultVariant: Variant;
  variant: Variant;
  setVariant: (v: Variant) => void;

  resetLayout: () => void;
}

const LayoutContext = createContext<LayoutContextValue | null>(null);

const COOKIE_MODE = "layout_mode";
const COOKIE_COLLAPSIBLE = "layout_collapsible";
const COOKIE_VARIANT = "layout_variant";

const DEFAULT_MODE: LayoutMode = "header";
const DEFAULT_COLLAPSIBLE: Collapsible = "icon";
const DEFAULT_VARIANT: Variant = "floating";

const MODES: readonly LayoutMode[] = ["header", "sidebar"];
const COLLAPSIBLES: readonly Collapsible[] = ["offcanvas", "icon", "none"];
const VARIANTS: readonly Variant[] = ["inset", "sidebar", "floating"];

function readCookie<T extends string>(name: string, fallback: T, allowed: readonly T[]): T {
  const v = getCookie(name);
  return v && (allowed as readonly string[]).includes(v) ? (v as T) : fallback;
}

export interface LayoutProviderProps {
  children: ReactNode;
  defaultMode?: LayoutMode;
  defaultCollapsible?: Collapsible;
  defaultVariant?: Variant;
}

export function LayoutProvider({
  children,
  defaultMode = DEFAULT_MODE,
  defaultCollapsible = DEFAULT_COLLAPSIBLE,
  defaultVariant = DEFAULT_VARIANT,
}: LayoutProviderProps) {
  const [mode, _setMode] = useState<LayoutMode>(() =>
    readCookie(COOKIE_MODE, defaultMode, MODES),
  );
  const [collapsible, _setCollapsible] = useState<Collapsible>(() =>
    readCookie(COOKIE_COLLAPSIBLE, defaultCollapsible, COLLAPSIBLES),
  );
  const [variant, _setVariant] = useState<Variant>(() =>
    readCookie(COOKIE_VARIANT, defaultVariant, VARIANTS),
  );

  const setMode = (m: LayoutMode) => {
    setCookie(COOKIE_MODE, m);
    _setMode(m);
  };
  const setCollapsible = (c: Collapsible) => {
    setCookie(COOKIE_COLLAPSIBLE, c);
    _setCollapsible(c);
  };
  const setVariant = (v: Variant) => {
    setCookie(COOKIE_VARIANT, v);
    _setVariant(v);
  };

  const toggle = () => setMode(mode === "header" ? "sidebar" : "header");

  const resetLayout = () => {
    setMode(defaultMode);
    setCollapsible(defaultCollapsible);
    setVariant(defaultVariant);
  };

  return (
    <LayoutContext.Provider
      value={{
        defaultMode,
        mode,
        setMode,
        toggle,
        defaultCollapsible,
        collapsible,
        setCollapsible,
        defaultVariant,
        variant,
        setVariant,
        resetLayout,
      }}
    >
      {children}
    </LayoutContext.Provider>
  );
}

export function useLayout(): LayoutContextValue {
  const ctx = useContext(LayoutContext);
  if (!ctx) throw new Error("useLayout must be used inside LayoutProvider");
  return ctx;
}
