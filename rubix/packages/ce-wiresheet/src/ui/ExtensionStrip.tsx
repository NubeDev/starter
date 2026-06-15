// The outer (extension-level) tab strip. One tab per loaded extension; the
// active extension's UIs render in the inner strip (UiTabHost). Two presentations:
//   - "edge"   → vertical rounded tabs on the screen's right edge (drawer closed)
//   - "inline" → a slim column inside the open drawer, left of the UI strip
// See ../../SDUI_UNIFIED_DESIGN.md §10.

import { Cpu, Bell, Boxes } from "lucide-react";
import type { ExtensionUi } from "../lib/ui/types";

const EXT_ICONS: Record<string, typeof Cpu> = { cpu: Cpu, bell: Bell };

export interface ExtensionStripProps {
  extensions: ExtensionUi[];
  activeId: string | null;
  onSelect: (id: string) => void;
  variant: "edge" | "inline";
}

export function ExtensionStrip({ extensions, activeId, onSelect, variant }: ExtensionStripProps) {
  const edge = variant === "edge";
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: edge ? 6 : 2,
        padding: edge ? 0 : 4,
        ...(edge
          ? {}
          : { borderRight: "1px solid #2c313c", background: "#0f1217", flexShrink: 0 }),
      }}
    >
      {extensions.map((e) => {
        const Icon = EXT_ICONS[e.icon ?? ""] ?? Boxes;
        const on = e.id === activeId;
        return (
          <button
            key={e.id}
            title={e.label}
            onClick={() => onSelect(e.id)}
            style={
              edge
                ? {
                    width: 26,
                    height: 56,
                    background: "rgba(20,23,30,0.92)",
                    border: "1px solid #2c313c",
                    borderRight: "none",
                    borderRadius: "6px 0 0 6px",
                    color: on ? "#e6e8eb" : "#cbd3e0",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }
                : {
                    width: 30,
                    height: 30,
                    borderRadius: 4,
                    border: "none",
                    cursor: "pointer",
                    color: on ? "#e6e8eb" : "#5a6172",
                    background: on ? "#2c313c" : "transparent",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }
            }
          >
            <Icon size={16} />
          </button>
        );
      })}
    </div>
  );
}
