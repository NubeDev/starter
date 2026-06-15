// The right-drawer tab host. Loads the UI list once (getUiManifest, stubbed),
// allocates one tab per UI, renders the active view full-bleed. Tab switching is
// manual — selecting a component never changes the active tab. Each UI declares
// a selection mode; `follow`/`sync` bind the view to the host's selected
// component. See ../../SDUI_UNIFIED_DESIGN.md §10.

import { useEffect, useMemo, useState } from "react";
import { Table, LayoutPanelLeft, Calendar, SquareDashed } from "lucide-react";
import type { ActionDef, UiEntry } from "../lib/ui/types";
import type { FlexValue } from "../lib/engine-types";
import { RenderWidget, type RenderCtx } from "./registry";
import { CollectionWidget } from "./CollectionWidget";
import "./widgets"; // register text/value/button

const ICONS: Record<string, typeof Table> = {
  table: Table,
  layout: LayoutPanelLeft,
  calendar: Calendar,
};

export interface UiTabHostProps {
  /** the active extension's UIs (one inner tab each) */
  uis: UiEntry[];
  currentParentUid: number;
  selectedUids: number[];
  onSelect: (uids: number[]) => void;
  onDrillIn?: (uid: number) => void;
  onNameContextMenu?: (uid: number, x: number, y: number) => void;
  onRowsChange?: (uids: number[]) => void;
  canGoUp?: boolean;
  onUp?: () => void;
  onSetDefault?: (componentUid: number, property: string, value: FlexValue) => void;
  onSetOverride?: (componentUid: number, property: string, value: FlexValue, duration: number) => void;
  onClearOverride?: (componentUid: number, property: string) => void;
  onAction?: (action: ActionDef, ctx: RenderCtx) => void;
}

export function UiTabHost({ uis, ...props }: UiTabHostProps) {
  const [activeId, setActiveId] = useState<string | null>(uis[0]?.id ?? null);

  // Keep the active tab valid as the extension (its UI set) changes.
  useEffect(() => {
    setActiveId((cur) => (uis.some((u) => u.id === cur) ? cur : uis[0]?.id ?? null));
  }, [uis]);

  const active = useMemo(() => uis.find((u) => u.id === activeId) ?? null, [uis, activeId]);
  const manifest = { uis };

  return (
    <div style={{ display: "flex", height: "100%", minHeight: 0 }}>
      {/* vertical tab strip */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 2,
          padding: 4,
          borderRight: "1px solid #2c313c",
          background: "#15181e",
          flexShrink: 0,
        }}
      >
        {manifest.uis.map((u) => {
          const Icon = ICONS[u.icon ?? ""] ?? SquareDashed;
          const on = u.id === activeId;
          return (
            <button
              key={u.id}
              title={u.label}
              onClick={() => setActiveId(u.id)}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 30,
                height: 30,
                borderRadius: 4,
                border: "none",
                cursor: "pointer",
                color: on ? "#e6e8eb" : "#5a6172",
                background: on ? "#2c313c" : "transparent",
              }}
            >
              <Icon size={16} />
            </button>
          );
        })}
      </div>
      {/* active view, full-bleed */}
      <div style={{ flex: 1, minWidth: 0, minHeight: 0 }}>
        {active && <ViewBody entry={active} {...props} />}
      </div>
    </div>
  );
}

function ViewBody({ entry, ...props }: { entry: UiEntry } & Omit<UiTabHostProps, "uis">) {
  const { view, selection } = entry;
  // follow/sync bind to the selected component; ignore/drive don't read it.
  const boundUid =
    selection === "follow" || selection === "sync" ? props.selectedUids[0] : undefined;
  const ctx: RenderCtx = { componentUid: boundUid, onAction: props.onAction };

  if (view.type === "collection") {
    const writesSelection = selection === "sync" || selection === "drive";
    return (
      <CollectionWidget
        currentParentUid={props.currentParentUid}
        selectedUids={props.selectedUids}
        onSelect={writesSelection ? props.onSelect : undefined}
        onDrillIn={props.onDrillIn}
        onNameContextMenu={props.onNameContextMenu}
        onRowsChange={props.onRowsChange}
        canGoUp={props.canGoUp}
        onUp={props.onUp}
        onSetDefault={props.onSetDefault}
        onSetOverride={props.onSetOverride}
        onClearOverride={props.onClearOverride}
      />
    );
  }

  if (view.type === "layout") {
    return (
      <div style={{ padding: 10, height: "100%", overflow: "auto" }}>
        {view.children.map((w, i) => (
          <RenderWidget key={`${w.type}:${i}`} node={w} ctx={ctx} />
        ))}
      </div>
    );
  }

  return (
    <div style={{ padding: 10, color: "#5a6172", fontSize: 12 }}>
      {view.type} view — not yet implemented
    </div>
  );
}
