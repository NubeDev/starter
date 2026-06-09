import { createContext, useContext, useState, type ReactNode } from "react";

import { getCookie, setCookie } from "@/lib/cookie";

export type SidebarVariant = "sidebar" | "floating" | "inset";
export type SidebarCollapsible = "offcanvas" | "icon" | "none";

const VARIANT_COOKIE = "layout_variant";
const COLLAPSIBLE_COOKIE = "layout_collapsible";

// Nexus opens with a floating, icon-collapsible sidebar — the detached,
// rounded macOS look. The user can switch both at runtime; the choice
// persists to a cookie so it survives reloads.
const DEFAULT_VARIANT: SidebarVariant = "floating";
const DEFAULT_COLLAPSIBLE: SidebarCollapsible = "icon";

interface LayoutContextValue {
  variant: SidebarVariant;
  setVariant: (v: SidebarVariant) => void;
  collapsible: SidebarCollapsible;
  setCollapsible: (c: SidebarCollapsible) => void;
  resetLayout: () => void;
  defaultVariant: SidebarVariant;
  defaultCollapsible: SidebarCollapsible;
}

const LayoutContext = createContext<LayoutContextValue | null>(null);

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [variant, _setVariant] = useState<SidebarVariant>(
    () => (getCookie(VARIANT_COOKIE) as SidebarVariant) || DEFAULT_VARIANT,
  );
  const [collapsible, _setCollapsible] = useState<SidebarCollapsible>(
    () =>
      (getCookie(COLLAPSIBLE_COOKIE) as SidebarCollapsible) ||
      DEFAULT_COLLAPSIBLE,
  );

  const setVariant = (v: SidebarVariant) => {
    _setVariant(v);
    setCookie(VARIANT_COOKIE, v);
  };
  const setCollapsible = (c: SidebarCollapsible) => {
    _setCollapsible(c);
    setCookie(COLLAPSIBLE_COOKIE, c);
  };
  const resetLayout = () => {
    setVariant(DEFAULT_VARIANT);
    setCollapsible(DEFAULT_COLLAPSIBLE);
  };

  return (
    <LayoutContext.Provider
      value={{
        variant,
        setVariant,
        collapsible,
        setCollapsible,
        resetLayout,
        defaultVariant: DEFAULT_VARIANT,
        defaultCollapsible: DEFAULT_COLLAPSIBLE,
      }}
    >
      {children}
    </LayoutContext.Provider>
  );
}

export function useLayout(): LayoutContextValue {
  const ctx = useContext(LayoutContext);
  if (!ctx) throw new Error("useLayout must be used within a LayoutProvider");
  return ctx;
}
