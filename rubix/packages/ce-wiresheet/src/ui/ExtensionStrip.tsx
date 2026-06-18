// The outer (extension-level) tab strip. One tab per loaded extension; the
// active extension's UIs render in the inner strip (UiTabHost). Two presentations:
//   - "edge"   → vertical rounded tabs on the screen's right edge (drawer closed)
//   - "inline" → a slim column inside the open drawer, left of the UI strip
// See ../../SDUI_UNIFIED_DESIGN.md §10.

import { Cpu, Bell, Boxes, Braces, Calendar } from "lucide-react";
import type { ExtensionUi } from "../lib/ui/types";

const EXT_ICONS: Record<string, typeof Cpu> = {
  cpu: Cpu,
  bell: Bell,
  calendar: Calendar,
  // JS / script extension. `Braces` ({ }) reads as code and stays distinct from
  // the inner Script tab's `Code2` (</>). Swap for FileCode2/SquareCode/Terminal
  // if you prefer.
  code: Braces,
  braces: Braces,
};

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
      style={
        edge
          ? {
              // One connected strip (not separate pills): shared surface,
              // rounded only on the outer (screen) edge, dividers between tabs.
              display: "flex",
              flexDirection: "column",
              background: "rgba(20,23,30,0.95)",
              border: "1px solid #2c313c",
              borderRight: "none",
              borderRadius: "8px 0 0 8px",
              overflow: "hidden",
              boxShadow: "0 1px 6px rgba(0,0,0,0.25)",
            }
          : {
              display: "flex",
              flexDirection: "column",
              gap: 2,
              padding: 4,
              borderRight: "1px solid #2c313c",
              background: "#0f1217",
              flexShrink: 0,
            }
      }
    >
      {extensions.map((e, i) => {
        const Icon = EXT_ICONS[e.icon ?? ""] ?? Boxes;
        const on = e.id === activeId;
        return (
          <button
            key={e.id}
            title={e.label}
            aria-label={e.label}
            onClick={() => onSelect(e.id)}
            style={
              edge
                ? {
                    width: 30,
                    height: 42,
                    border: "none",
                    borderTop: i === 0 ? "none" : "1px solid #2c313c",
                    // active accent on the inner edge; tab "connects" into the drawer
                    boxShadow: on ? "inset 2px 0 0 var(--ce-accent, #3b6eff)" : "none",
                    background: on ? "#1d2128" : "transparent",
                    color: on ? "#e6e8eb" : "#8892a0",
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
