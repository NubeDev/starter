// Left nav rail — icon-only column, sticky inside the page, tooltip
// on hover, accent glow on the active row. No URL routing for v1; a
// local `section` state lets the page show how the rail behaves.

import * as React from "react";
import {
  LayoutDashboard,
  Sun,
  Wind,
  Droplets,
  Mountain,
  BatteryCharging,
  Settings,
} from "lucide-react";

import type { SourceKind } from "./mock-data";
import { ACCENT } from "./mock-data";

export type NavSection =
  | "overview"
  | "solar"
  | "wind"
  | "water"
  | "hydro"
  | "storage"
  | "settings";

interface NavItem {
  id: NavSection;
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  /** Source kind for accent colour. `undefined` for overview/settings. */
  kind?: SourceKind;
}

const ITEMS: ReadonlyArray<NavItem> = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "solar",    label: "Solar",    icon: Sun,             kind: "solar"   },
  { id: "wind",     label: "Wind",     icon: Wind,            kind: "wind"    },
  { id: "water",    label: "Water",    icon: Droplets,        kind: "water"   },
  { id: "hydro",    label: "Hydro",    icon: Mountain,        kind: "hydro"   },
  { id: "storage",  label: "Storage",  icon: BatteryCharging, kind: "storage" },
  { id: "settings", label: "Settings", icon: Settings },
];

export function NavRail({
  active,
  onSelect,
}: {
  active: NavSection;
  onSelect: (id: NavSection) => void;
}): React.ReactElement {
  return (
    <nav
      aria-label="Energy sections"
      className="sticky top-2 flex w-16 shrink-0 flex-col items-center gap-1.5 rounded-2xl border border-white/10 bg-slate-950/70 p-2"
    >
      {ITEMS.map((item) => {
        const Icon = item.icon;
        const isActive = active === item.id;
        const accentCss = item.kind ? ACCENT[item.kind].from : "#CA8A04";
        return (
          <button
            key={item.id}
            type="button"
            onClick={() => onSelect(item.id)}
            aria-label={item.label}
            aria-current={isActive ? "page" : undefined}
            title={item.label}
            className={
              "group relative flex h-11 w-11 cursor-pointer items-center justify-center rounded-xl " +
              "text-slate-400 transition-colors duration-200 " +
              "hover:bg-white/5 hover:text-white " +
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40 " +
              (isActive ? "nrg-nav-active text-white" : "")
            }
            style={
              isActive
                ? ({ ["--nrg-nav-accent" as never]: accentCss } as React.CSSProperties)
                : undefined
            }
          >
            <Icon size={18} className="shrink-0" />
            {/* Tooltip */}
            <span
              role="tooltip"
              className={
                "pointer-events-none absolute left-full top-1/2 z-30 ml-3 -translate-y-1/2 " +
                "whitespace-nowrap rounded-md border border-white/10 bg-slate-900/95 px-2 py-1 " +
                "text-xs font-medium text-white opacity-0 shadow-lg backdrop-blur " +
                "transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100"
              }
            >
              {item.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
