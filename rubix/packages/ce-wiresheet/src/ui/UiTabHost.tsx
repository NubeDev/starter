// The right-drawer tab host. Loads the UI list once (getUiManifest, stubbed),
// allocates one tab per UI, renders the active view full-bleed. Tab switching is
// manual — selecting a component never changes the active tab. Each UI declares
// a selection mode; `follow`/`sync` bind the view to the host's selected
// component. See ../../SDUI_UNIFIED_DESIGN.md §10.

import { useEffect, useMemo, useState } from "react";
import { useStructural } from "../lib/store";
import { Table, LayoutPanelLeft, Calendar, ListTree, Timer, Repeat, SquareDashed } from "lucide-react";
import type { UiEntry } from "../lib/ui/types";
import type { FlexValue } from "../lib/engine-types";
import { RenderWidget, type RenderCtx } from "./registry";
import { CollectionWidget } from "./CollectionWidget";
import { TreeWidget } from "./TreeWidget";
import { injectUiStyles } from "./styles";
import "./widgets"; // register text/value/button
import "./SchedulePanel"; // register schedule

injectUiStyles();

const ICONS: Record<string, typeof Table> = {
  table: Table,
  layout: LayoutPanelLeft,
  calendar: Calendar,
  tree: ListTree,
  timer: Timer,
  cron: Repeat,
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
  onCallAction?: (componentUid: number, name: string, params?: Record<string, FlexValue>) => Promise<Record<string, FlexValue>>;
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
    <div className="ce-ui-root" style={{ display: "flex", height: "100%", minHeight: 0 }}>
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
        {active && <ViewBody entry={active} uis={uis} onPickUi={setActiveId} {...props} />}
      </div>
    </div>
  );
}

function ViewBody({
  entry,
  uis,
  onPickUi,
  ...props
}: { entry: UiEntry; uis: UiEntry[]; onPickUi: (id: string) => void } & Omit<UiTabHostProps, "uis">) {
  const { view, selection, appliesTo } = entry;
  // follow/sync bind to the selected component; ignore/drive don't read it.
  const wantsSel = selection === "follow" || selection === "sync";
  const selUid = wantsSel ? props.selectedUids[0] : undefined;
  const selType = useStructural((s) => (selUid != null ? s.components.get(selUid)?.type : undefined));
  // When a UI targets a component type, only bind to a matching selection.
  const typeOk = !appliesTo || (!!selType && selType.toLowerCase().includes(appliesTo.toLowerCase()));
  const boundUid = typeOk ? selUid : undefined;

  if (appliesTo && wantsSel && boundUid == null) {
    // Empty state: one column per type-bound UI in this extension. Clicking a
    // component selects it and jumps to that type's tab.
    const typeBound = uis.filter((u) => u.appliesTo);
    return <MultiTypePicker uis={typeBound} currentParentUid={props.currentParentUid} onSelect={props.onSelect} onPickUi={onPickUi} />;
  }

  const ctx: RenderCtx = { componentUid: boundUid, callAction: props.onCallAction };

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

  if (view.type === "tree") {
    const writesSelection = selection === "sync" || selection === "drive";
    return (
      <TreeWidget
        currentParentUid={props.currentParentUid}
        selectedUids={props.selectedUids}
        onSelect={writesSelection ? props.onSelect : undefined}
        onDrillIn={props.onDrillIn}
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

/** Empty-state: one column per type-bound UI, each listing the folder's matching
 *  components. Clicking selects the component and jumps to that type's tab. */
function MultiTypePicker({
  uis,
  currentParentUid,
  onSelect,
  onPickUi,
}: {
  uis: UiEntry[];
  currentParentUid: number;
  onSelect: (uids: number[]) => void;
  onPickUi: (id: string) => void;
}) {
  const components = useStructural((s) => s.components);
  const inFolder = useMemo(
    () => [...components.values()].filter((c) => c.parent === currentParentUid).sort((a, b) => a.name.localeCompare(b.name)),
    [components, currentParentUid],
  );
  return (
    <div style={{ display: "flex", height: "100%", minHeight: 0 }}>
      {uis.map((u) => {
        const t = (u.appliesTo ?? "").toLowerCase();
        const matches = inFolder.filter((c) => c.type.toLowerCase().includes(t));
        return (
          <div key={u.id} style={{ flex: 1, minWidth: 0, borderRight: "1px solid #2c313c", display: "flex", flexDirection: "column" }}>
            <div style={{ padding: "8px 10px", color: "#cbd3e0", fontSize: 12, fontWeight: 500, borderBottom: "1px solid #2c313c", flexShrink: 0 }}>
              {u.label} <span style={{ color: "#5a6172" }}>({matches.length})</span>
            </div>
            <div style={{ flex: 1, minHeight: 0, overflow: "auto", padding: 6, display: "flex", flexDirection: "column", gap: 4 }}>
              {matches.length === 0 ? (
                <div style={{ color: "#5a6172", fontSize: 11, padding: 6 }}>none here</div>
              ) : (
                matches.map((c) => (
                  <button
                    key={c.uid}
                    onClick={() => { onSelect([c.uid]); onPickUi(u.id); }}
                    style={{ display: "flex", alignItems: "baseline", gap: 8, padding: "6px 8px", textAlign: "left", background: "#1a1d24", border: "1px solid #2c313c", borderRadius: 4, cursor: "pointer", color: "#e6e8eb", fontSize: 12 }}
                  >
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.name}</span>
                  </button>
                ))
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
