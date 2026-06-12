import { createContext, memo, useContext, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Handle, Position, useStore as useRfStore } from "@xyflow/react";
import { useShallow } from "zustand/react/shallow";
import {
  useValues,
  useSchemaVersion,
  useStructural,
  useStatusFlags,
  propertyDataType,
} from "../lib/store";
import { diagRecordRender } from "../lib/diagnostics";
import { usePresence, PRESENCE_PALETTE } from "../lib/presence";
import {
  STATUS_OVERRIDDEN,
  CATEGORY_INPUT,
  CATEGORY_OUTPUT,
  CATEGORY_CONFIG,
  DATATYPE_NUMBER,
  DATATYPE_BOOL,
  DATATYPE_STRING,
  ROLE_NORMAL,
  ROLE_STATUS,
  type Component,
  type Edge as EdgeT,
  type Property,
  type PropertyCategory,
  type PropertyDataType,
  type PropertySystemRole,
} from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import {
  facetFor,
  rawFacet,
  aliasLabel,
  exposedPorts,
  FACET_PROP,
  type PropFacet,
} from "../lib/facet";

// Editor-level capabilities the ConnectPicker needs for its "New" flow — the
// creatable component types and a "create one in the current folder" action.
// Supplied by CeEditor via context (crosses the picker's createPortal).
export interface CeWiresheetCtx {
  componentTypes: Array<{ name: string; type: string; group: string }>;
  createComponent: (
    type: string,
    opts?: { nearUid?: number; side?: "left" | "right" },
  ) => Promise<Component | null>;
  // Add an edge and update the view incrementally (append in-folder; reload only
  // for a cross-folder target that needs a ghost). Avoids a full reload per link.
  connectEdge: (payload: {
    sourceUid: number;
    sourcePropUid: number;
    targetUid: number;
    targetPropUid: number;
  }) => Promise<void>;
  // Expose a child's prop as a port on the current container (folder). Present
  // only when inside a container (not at root); parentName is that container's
  // display name for the menu label.
  parentName?: string;
  exposeProp?: (
    childPropUid: number,
    childComponentUid: number,
    side: "input" | "output",
    defaultLabel: string,
  ) => void | Promise<void>;
  // Remove an exposed port from `folderUid`'s __facets (the folder the port is on).
  unexposeProp?: (folderUid: number, childPropUid: number) => void | Promise<void>;
  // Open the Details panel for any component (e.g. the off-canvas child behind an
  // exposed port, so its facet — the source of truth — can be edited there).
  openDetails?: (componentUid: number) => void;
  // Request a (debounced) scope reload. Used when a component's live __facets
  // stream changes its EXPOSED-PORT set (expose/unexpose from another session):
  // that alters port handles and thus edge routing, so ghosts/ports must be
  // rebuilt — a row re-derive isn't enough. Cosmetic facet edits don't call this.
  requestReload?: () => void;
}
export const CeWiresheetContext = createContext<CeWiresheetCtx | null>(null);

// Composite view of a property assembled from REST (structure + statusFlags
// snapshot) and WS schema (dataType). One row per non-CONFIG, non-system Property.
interface PropRow {
  uid: number;
  name: string;
  category: PropertyCategory;
  dataType: PropertyDataType;
  systemRole?: PropertySystemRole;
  facet?: PropFacet; // per-prop presentation metadata from __facets
  exposed?: boolean; // a child prop projected onto this (parent) as a port
  exposedComponent?: number; // for an exposed port, the child component that owns it
  facetPropUid?: number; // for an exposed port, the child's __facets prop uid (live)
}

export type FunctionBlockData = {
  componentUid: number;
  // Display name from REST (e.g. "add", "Heartbeat1"). Shown in the title bar; the
  // component TYPE is shown below it (smaller). Both come from REST since the WS
  // schema only carries `kind` (= type), not the instance name.
  name?: string;
  // True if this component has children — drives the "↵ enter" affordance and a small
  // badge in the title bar. Filled in by App.tsx from the REST `childrenCount` field.
  hasChildren?: boolean;
  childCount?: number;
  // True if this component's TYPE declares any actions (from /schema). Drives the
  // ⚡ marker in the bottom lip. Filled in by App.tsx from the action index.
  hasActions?: boolean;
  // Click-into handler. Provided by App.tsx so the block doesn't have to know about
  // routing/breadcrumb state.
  onEnter?: (uid: number) => void;
  // Node-level right-click handler. Provided by App.tsx so it can open a menu
  // that operates on the current multi-selection (reparent etc.). Property rows
  // intercept their own onContextMenu so this only fires on the node body
  // (title bar + spacing).
  onContextMenu?: (uid: number, x: number, y: number) => void;
} & Record<string, unknown>;

const COLOR_NUMBER = "#4a9eff";
const COLOR_BOOL = "#4ade80";
const COLOR_STRING = "#f59e0b";

// Two stacked text lines (12px name + 10px type) need ~32px with default line
// heights; the title bar pads 4px top + 4px bottom, so the content box must be
// at least 32px → 40px outer height. Less and the type label's descenders get
// clipped by overflow: hidden on the node root.
const TITLE_H = 40;
const ROW_H = 18;
export const NODE_W = 220;

// Height of the GhostNode (sub-node) that represents an off-canvas component
// endpoint of a cross-folder edge — exactly one property row so it lines up
// flush with the source/target prop on the visible component. Width is
// computed per-ghost from the content (see ghostWidthFor) instead of using a
// fixed value, so a short label like "root · out" doesn't render a half-empty
// box of padding.
export const GHOST_H = ROW_H;
export const GHOST_W_MIN = 90;
export const GHOST_W_MAX = 260;

// Estimate width needed for `<path> · <propName>` rendered in the same 10px
// monospace font + padding the ghost uses. Slight overshoot (6.2px/char)
// since glyph widths vary; ellipsis handles any remaining overflow.
export function ghostWidthFor(path: string, propName: string): number {
  const text = `${path || "root"} · ${propName}`;
  // 22px = horizontal padding (8 + 8) + handle marker (8) − a couple px the
  // marker overlaps the edge.
  const w = 22 + Math.ceil(text.length * 6.2);
  return Math.max(GHOST_W_MIN, Math.min(GHOST_W_MAX, w));
}

// Drop the leading "root/" (or bare "root") from a component path. Every path
// starts at root, so the prefix is noise — labels read more cleanly without
// it. Used both for ghost labels and the popover list rows so the same path
// formatting is applied everywhere a cross-folder location is shown.
export function stripRoot(path: string): string {
  if (path === "root" || path === "") return "root";
  if (path.startsWith("root/")) return path.slice(5);
  return path;
}

// Human-readable dataType label (tooltips only; never compared).
const DATATYPE_LABEL: Record<number, string> = {
  [DATATYPE_NUMBER]: "number",
  [DATATYPE_BOOL]: "bool",
  [DATATYPE_STRING]: "string",
};

// Below this zoom the graph is a far overview — individual values aren't
// legible, so nodes render in a cheap level-of-detail form: title + handles
// only, no property rows / value cells, and the value/status subscriptions go
// dormant (no per-frame re-renders). Cuts DOM + reconcile cost dramatically
// when the whole graph (100s of nodes) is on screen. Kept low (0.12) so full
// detail stays through normal working zooms and LOD only kicks in on a deep
// zoom-out.
const LOD_ZOOM = 0.12;
// Stable empties returned by the value/status selectors while in LOD, so a
// value change doesn't re-render a node that isn't showing values anyway.
const EMPTY_VALUES: Record<number, DecodedValue | undefined> = Object.freeze({});
const EMPTY_FLAGS: Record<number, number> = Object.freeze({});

function colorForType(dt: PropertyDataType): string {
  if (dt === DATATYPE_BOOL) return COLOR_BOOL;
  if (dt === DATATYPE_STRING) return COLOR_STRING;
  return COLOR_NUMBER;
}

// Fallback for properties not in the WS schema table (CONFIG-category and
// NULL-typed are excluded from the value plane, so the schema doesn't carry a
// dataType for them). Infer from the REST value's runtime type.
function inferDataType(v: unknown): PropertyDataType {
  if (typeof v === "boolean") return DATATYPE_BOOL;
  if (typeof v === "string") return DATATYPE_STRING;
  return DATATYPE_NUMBER;
}

function fmtValue(v: DecodedValue | undefined, dt: PropertyDataType): string {
  if (v === undefined) return "—";
  if (typeof v === "bigint") return v.toString();
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "string") return JSON.stringify(v).slice(1, -1);
  // number
  if (dt === DATATYPE_BOOL) return v ? "true" : "false";
  if (Number.isInteger(v)) return v.toString();
  return v.toFixed(2);
}

// fmtValue + facet: alias label wins; otherwise apply the facet's decimals and
// unit suffix on top of the base formatting.
function fmtValueFacet(
  v: DecodedValue | undefined,
  dt: PropertyDataType,
  facet: PropFacet | undefined,
): string {
  const al = aliasLabel(facet?.aliases, v);
  if (al != null) return al;
  let base: string;
  if (facet?.decimals != null && typeof v === "number") base = v.toFixed(facet.decimals);
  else base = fmtValue(v, dt);
  return facet?.unit && base !== "—" ? `${base} ${facet.unit}` : base;
}

function rowYCenter(rowIndex: number): number {
  return TITLE_H + rowIndex * ROW_H + ROW_H / 2;
}

// A uid rendered as a click-to-copy chip (menus aren't selection-friendly — the
// app uses user-select:none and the menu dismisses on pointerdown).
export function CopyUid({ label, value }: { label: string; value: number }) {
  const [copied, setCopied] = useState(false);
  return (
    <span
      onClick={(e) => {
        e.stopPropagation();
        void navigator.clipboard?.writeText(String(value)).then(
          () => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 900);
          },
          () => {},
        );
      }}
      title="click to copy"
      style={{
        cursor: "pointer",
        textDecoration: "underline dotted",
        color: copied ? "#7ee787" : "inherit",
      }}
    >
      {label} {copied ? "copied" : value}
    </span>
  );
}

// Right-click menu for a property row. Set / clear an override, or initiate an
// edge from this property via "Connect to…".
function PropertyContextMenu({
  x,
  y,
  propName,
  propUid,
  category,
  dataType,
  currentValue,
  overridden,
  exposed,
  portOwner,
  componentUid,
  onClose,
}: {
  x: number;
  y: number;
  propName: string;
  propUid: number;
  category: PropertyCategory;
  dataType: PropertyDataType;
  currentValue: DecodedValue | undefined;
  overridden: boolean;
  exposed?: boolean;
  portOwner?: number;
  componentUid: number;
  onClose: () => void;
}) {
  const [promptOpen, setPromptOpen] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [draft, setDraft] = useState<string>(
    currentValue == null ? "" : typeof currentValue === "string" ? currentValue : String(currentValue),
  );
  // Override duration in seconds. 0 = permanent (until cleared). Default to 1 minute
  // — a reasonable "I want to nudge this for a moment" length.
  const [durationSec, setDurationSec] = useState<number>(60);
  const ctx = useContext(CeWiresheetContext);

  useEffect(() => {
    const dismiss = (e: Event) => {
      const el = e.target as Element | null;
      // The picker carries its own data-ce-menu so its clicks are also
      // tolerated here — clicking inside it should NOT dismiss the menu.
      if (el && el.closest("[data-ce-menu]")) return;
      onClose();
    };
    // Capture-phase pointerdown: React Flow's pane stopImmediatePropagation's on
    // press, so a bubble-phase document mousedown never sees clicks on the canvas
    // (that's why click-away wasn't closing the menu). Capture fires first.
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  // Only edges between inputs and outputs make sense; config props don't
  // participate in dataflow edges, so "Connect to…" is hidden for them.
  const canConnect = category === CATEGORY_INPUT || category === CATEGORY_OUTPUT;
  // All normal properties can be overridden — including outputs, where the
  // override freezes the engine-computed value via PATCH /overrides. (The
  // inline click-to-edit on the row is still input/config only, since that
  // path PATCHes /nodes which wouldn't take on outputs.)
  // Exposed ports can't be overridden from here — override is name-based and the
  // row shows the port's label, not the child's real prop name (and the engine has
  // no prop-uid override yet). Override the real value inside the child component.
  const overridable =
    !exposed &&
    (category === CATEGORY_INPUT || category === CATEGORY_CONFIG || category === CATEGORY_OUTPUT);

  const parse = (raw: string): string | number | boolean | null => {
    const t = raw.trim();
    if (t === "") return null;
    if (dataType === DATATYPE_BOOL) {
      const lower = t.toLowerCase();
      return lower === "true" || lower === "1" || lower === "yes";
    }
    if (dataType === DATATYPE_STRING) return t;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  };

  // Optimistic update — flip the property's status bits locally BEFORE the
  // network call so the OVR badge / amber tint appears the moment the user
  // clicks. The real value/status arrives via the WS binary frame within a few
  // ms; without this, the visual lags the click by the full HTTP round trip.
  // The STATUS section in the next frame will overwrite our optimistic value
  // with the authoritative one.
  const optimisticSetBit = async (uid: number, bit: number, on: boolean) => {
    const { useStatusFlags } = await import("../lib/store");
    const s = useStatusFlags.getState();
    const cur = s.flags.get(uid) ?? 0;
    const next = on ? cur | bit : cur & ~bit;
    s.applyStatus([uid], [next]);
  };

  const setOverride = async () => {
    const parsed = parse(draft);
    if (parsed == null) {
      onClose();
      return;
    }
    onClose();
    const { useStructural } = await import("../lib/store");
    const cur = useStructural.getState().components.get(componentUid);
    const uid = cur?.properties[propName]?.uid;
    if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, true);
    try {
      const { patchOverrides } = await import("../lib/rest");
      const updated = await patchOverrides(componentUid, {
        setOverrides: [
          { property: propName, value: parsed, duration: durationSec },
        ],
      });
      useStructural.getState().upsertComponent(updated);
    } catch (e) {
      console.error("set override failed:", (e as Error).message);
      // Roll the optimistic flip back. The next WS frame's STATUS section will
      // reconcile authoritatively anyway, but this is faster.
      if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, false);
    }
  };

  const clearOverride = async () => {
    onClose();
    const { useStructural } = await import("../lib/store");
    const cur = useStructural.getState().components.get(componentUid);
    const uid = cur?.properties[propName]?.uid;
    if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, false);
    try {
      const { patchOverrides } = await import("../lib/rest");
      const updated = await patchOverrides(componentUid, { clearOverrides: [propName] });
      useStructural.getState().upsertComponent(updated);
    } catch (e) {
      console.error("clear override failed:", (e as Error).message);
      if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, true);
    }
  };

  return createPortal(
    <div
      data-ce-menu
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        left: x,
        top: y,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        padding: 4,
        minWidth: 180,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 11,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
      }}
    >
      <div
        style={{ padding: "4px 8px", color: "#8892a0", borderBottom: "1px solid #2c313c", marginBottom: 4 }}
      >
        {propName} <span style={{ color: "#5a6172" }}>· {dataType}</span>
        <div
          style={{
            fontSize: 9,
            color: "#5a6172",
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            marginTop: 2,
          }}
        >
          <CopyUid label="prop" value={propUid} /> · <CopyUid label="comp" value={componentUid} />
        </div>
      </div>
      {promptOpen ? (
        <div style={{ padding: "4px 6px", display: "flex", flexDirection: "column", gap: 4 }}>
          {dataType === DATATYPE_BOOL ? (
            <select
              autoFocus
              className="nodrag"
              value={draft || "true"}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") setOverride();
                else if (e.key === "Escape") onClose();
                e.stopPropagation();
              }}
              style={overrideInputStyle}
            >
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          ) : (
            <input
              autoFocus
              className="nodrag"
              type={dataType === DATATYPE_NUMBER ? "number" : "text"}
              inputMode={dataType === DATATYPE_NUMBER ? "decimal" : undefined}
              step={dataType === DATATYPE_NUMBER ? "any" : undefined}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") setOverride();
                else if (e.key === "Escape") onClose();
                e.stopPropagation();
              }}
              style={overrideInputStyle}
              placeholder="override value…"
            />
          )}
          <label style={{ display: "flex", alignItems: "center", gap: 4, color: "#8892a0", fontSize: 10 }}>
            <span style={{ flex: 1 }}>duration</span>
            <select
              className="nodrag"
              value={durationSec}
              onChange={(e) => setDurationSec(Number(e.target.value))}
              onClick={(e) => e.stopPropagation()}
              style={{
                background: "#0f1115",
                color: "#e6e8eb",
                border: "1px solid #2c313c",
                borderRadius: 2,
                padding: "2px 4px",
                fontSize: 11,
                fontFamily: "inherit",
              }}
            >
              <option value={10}>10 sec</option>
              <option value={30}>30 sec</option>
              <option value={60}>1 min</option>
              <option value={300}>5 min</option>
              <option value={1200}>20 min</option>
              <option value={3600}>1 hr</option>
              <option value={7200}>2 hr</option>
              <option value={86400}>24 hr</option>
              <option value={0}>permanent</option>
            </select>
          </label>
          <button
            onClick={setOverride}
            style={{
              padding: "3px 6px",
              background: "#3b6eff",
              color: "#fff",
              border: "1px solid #5a83ff",
              borderRadius: 2,
              cursor: "pointer",
              fontSize: 11,
              fontFamily: "inherit",
            }}
          >
            Set override
          </button>
        </div>
      ) : (
        <>
          {overridable && (
            <MenuItem
              onClick={() => setPromptOpen(true)}
              label={overridden ? "Change override…" : "Set override…"}
            />
          )}
          {overridable && overridden && (
            <MenuItem onClick={clearOverride} label="Clear override" danger />
          )}
          {canConnect && (
            <MenuItem onClick={() => setPickerOpen(true)} label="Connect to…" />
          )}
          {canConnect && ctx?.exposeProp && ctx.parentName && (
            <MenuItem
              onClick={() => {
                ctx.exposeProp?.(
                  propUid,
                  componentUid,
                  category === CATEGORY_OUTPUT ? "output" : "input",
                  propName,
                );
                onClose();
              }}
              label={`Expose on ${ctx.parentName}`}
            />
          )}
          {exposed && ctx?.openDetails && (
            <MenuItem
              onClick={() => {
                ctx.openDetails?.(componentUid);
                onClose();
              }}
              label="Configure…"
            />
          )}
          {exposed && ctx?.unexposeProp && portOwner != null && (
            <MenuItem
              onClick={() => {
                ctx.unexposeProp?.(portOwner, propUid);
                onClose();
              }}
              label="Un-expose"
              danger
            />
          )}
        </>
      )}
      {pickerOpen && (
        <ConnectPicker
          x={x}
          y={y}
          sourceComponentUid={componentUid}
          sourcePropUid={propUid}
          sourceCategory={category === CATEGORY_OUTPUT ? "output" : "input"}
          onClose={() => {
            setPickerOpen(false);
            onClose();
          }}
        />
      )}
    </div>,
    document.body,
  );
}

// Pops next to the property menu when the user clicks "Connect to…". Lists
// candidate target properties on every component in the current view (siblings
// of the source component), filtered by the source's category — outputs can
// only edge into inputs and vice versa. Self-component is skipped because an
// edge within the same component would be a no-op / cycle.
//
// On select: POST /edge with source on the OUTPUT side, target on the INPUT
// side (engine convention), then close. Topology event drives the reload so
// the new edge appears within a tick.
function ConnectPicker({
  x,
  y,
  sourceComponentUid,
  sourcePropUid,
  sourceCategory,
  onClose,
}: {
  x: number;
  y: number;
  sourceComponentUid: number;
  sourcePropUid: number;
  sourceCategory: "input" | "output";
  onClose: () => void;
}) {
  const [filter, setFilter] = useState("");
  // Which component is currently expanded. null = collapsed accordion (just
  // showing the component list). One-at-a-time so the picker stays compact.
  const [expanded, setExpanded] = useState<number | null>(null);
  // "New" mode: create a fresh component and connect to it, instead of picking
  // an existing one. Needs the editor's component types + create action.
  const ctx = useContext(CeWiresheetContext);
  const [creatingNew, setCreatingNew] = useState(false);
  // After picking a type in New mode, the freshly-created component is parked
  // here so the user can choose WHICH of its matching props to connect to,
  // instead of auto-wiring the first one.
  const [pendingNew, setPendingNew] = useState<Component | null>(null);
  // Keyboard navigation: index of the highlighted row (a property in Existing /
  // pick-input mode, a type in New mode). The filter is SHARED across Existing
  // and New, so switching (Tab / the +New button) keeps whatever you typed.
  const [highlight, setHighlight] = useState(0);
  const hlRef = useRef<HTMLButtonElement>(null);
  // Reset the highlight to the top whenever the candidate list changes (new
  // filter text, switching Existing↔New, or entering pick-input).
  useEffect(() => {
    setHighlight(0);
  }, [filter, creatingNew, pendingNew]);
  // Keep the highlighted row scrolled into view while arrowing through it.
  useEffect(() => {
    hlRef.current?.scrollIntoView({ block: "nearest" });
  }, [highlight, creatingNew]);

  // Dismiss on outside-click / Escape. Capture-phase pointerdown so React Flow's
  // pane (which stopImmediatePropagation's on press) can't swallow it. The
  // picker's root carries `data-ce-menu`, so clicks inside it don't dismiss.
  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-menu]")) return;
      onClose();
    };
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
      document.removeEventListener("keydown", onEsc);
    };
  }, [onClose]);
  // `sourceCategory` is the client-side direction ("input"/"output") of the
  // port the user is wiring FROM; the candidate category we want is the
  // opposite, as a numeric API category to compare against `p.category`.
  const wantCategory: PropertyCategory =
    sourceCategory === "output" ? CATEGORY_INPUT : CATEGORY_OUTPUT;

  // Edges can cross folders (per spec), so candidates include EVERY component
  // in the engine — not just siblings of the source. useStructural only holds
  // the current view's children, so we fetch the full tree on mount. Cached
  // inside the picker so reopening the same picker doesn't refetch.
  const [allComponents, setAllComponents] = useState<Component[] | null>(null);
  const [allEdges, setAllEdges] = useState<EdgeT[] | null>(null);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const { getRootNodes } = await import("../lib/rest");
      try {
        const resp = await getRootNodes({ depth: -1, nested: true, withEdges: true });
        if (cancelled) return;
        const flat: Component[] = [];
        const walk = (c: Component) => {
          flat.push(c);
          c.children?.forEach(walk);
        };
        // resp.nodes[0] is the root; we want its descendants (root itself isn't
        // a target — its properties are engine-managed indicators).
        const root = resp.nodes[0];
        root?.children?.forEach(walk);
        setAllComponents(flat);
        setAllEdges(resp.edges ?? []);
      } catch {
        // Fall back to the current view if the global fetch fails.
        if (cancelled) return;
        setAllComponents([...useStructural.getState().components.values()]);
        setAllEdges([...useStructural.getState().edges.values()]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Inputs in dataflow take at most one incoming edge (the source of truth for
  // their value). When the user is wiring from an output, hide inputs that
  // already have something connected — they can't accept another. Outputs can
  // fan out to many targets, so the reverse direction needs no such filter.
  // Match by the target property's UID (the engine provides
  // `targetPropertyUid` on every edge) — an integer compare, no property-name
  // string matching. A set of all currently-targeted input prop uids.
  const takenInputUids = new Set<number>();
  if (sourceCategory === "output" && allEdges) {
    for (const e of allEdges) {
      if (e.targetPropertyUid != null) takenInputUids.add(e.targetPropertyUid);
    }
  }

  interface Candidate {
    propUid: number;
    propName: string;
  }
  interface CompGroup {
    componentUid: number;
    componentName: string;
    path: string;
    sibling: boolean; // true when this component shares the source's parent
    isParent: boolean; // the source's own container (feed-through target)
    isChild: boolean; // nested inside the source component
    props: Candidate[];
  }

  // Look up the source component's parent so we can flag siblings. Use the
  // current view's structural cache — the source is always in scope there.
  const sourceComp = useStructural.getState().components.get(sourceComponentUid);
  const sourceParent = sourceComp?.parent;
  const sourceName = sourceComp?.name || "component";

  const groups: CompGroup[] = [];
  const componentList = allComponents ?? [];
  for (const c of componentList) {
    if (c.uid === sourceComponentUid) continue;
    // Parent / children are grouped + labelled distinctly, but they connect like
    // any other target: a normal opposite-category edge (the engine supports
    // cross-folder edges). Feed-through (same-category) edges aren't used.
    const isParent = sourceParent !== undefined && c.uid === sourceParent;
    const isChild = c.parent === sourceComponentUid;
    const props: Candidate[] = [];
    for (const [name, p] of Object.entries(c.properties)) {
      if (p.category !== wantCategory) continue;
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      if (takenInputUids.has(p.uid)) continue;
      props.push({ propUid: p.uid, propName: name });
    }
    if (props.length === 0) continue;
    props.sort((a, b) => a.propName.localeCompare(b.propName));
    groups.push({
      componentUid: c.uid,
      componentName: c.name || c.type,
      path: c.path,
      sibling: sourceParent !== undefined && c.parent === sourceParent,
      isParent,
      isChild,
      props,
    });
  }
  // Parent (feed-through) first, then siblings (alphabetical by name), then
  // everything else (alphabetical by path). Puts the most likely targets at the
  // top while still surfacing cross-folder options without scrolling past them.
  // Tier order: parent (0) → same level (1) → children (2) → everything else (3).
  const tierOf = (g: CompGroup) => (g.isParent ? 0 : g.sibling ? 1 : g.isChild ? 2 : 3);
  groups.sort((a, b) => {
    const ta = tierOf(a);
    const tb = tierOf(b);
    if (ta !== tb) return ta - tb;
    // Within "other" sort by path; otherwise by name.
    return ta === 3
      ? a.path.localeCompare(b.path)
      : a.componentName.localeCompare(b.componentName);
  });

  const f = filter.trim().toLowerCase();
  // A path-style filter ("add1/add2/ad") splits at the LAST slash into a folder
  // SCOPE ("add1/add2") that the component's path must contain, and a TERM
  // ("ad") matched against the component name or the path tail BELOW that scope.
  // So it finds matches in that folder AND deeper — not just direct children —
  // and the term doesn't accidentally match folder names in the scope itself.
  const slash = f.lastIndexOf("/");
  const pathScope = slash >= 0 ? f.slice(0, slash) : "";
  const term = slash >= 0 ? f.slice(slash + 1) : f;

  const filteredGroups: CompGroup[] = !f
    ? groups
    : groups
        .map((g) => {
          const path = g.path.toLowerCase();
          if (pathScope && !path.includes(pathScope)) return null;
          if (!term) return g; // pure folder scope → whole group qualifies
          const tail = pathScope ? path.slice(path.indexOf(pathScope) + pathScope.length) : path;
          if (g.componentName.toLowerCase().includes(term) || tail.includes(term)) return g;
          const props = g.props.filter((p) => p.propName.toLowerCase().includes(term));
          return props.length > 0 ? { ...g, props } : null;
        })
        .filter((g): g is CompGroup => g !== null);

  const create = async (target: { componentUid: number; propUid: number }) => {
    // Engine convention: source = output side, target = input side. Flip based
    // on which end the user is wiring from.
    const payload =
      sourceCategory === "output"
        ? {
            sourceUid: sourceComponentUid,
            sourcePropUid,
            targetUid: target.componentUid,
            targetPropUid: target.propUid,
          }
        : {
            sourceUid: target.componentUid,
            sourcePropUid: target.propUid,
            targetUid: sourceComponentUid,
            targetPropUid: sourcePropUid,
          };
    try {
      // Incremental edge add (append in-folder, reload only for cross-folder).
      // Falls back to a plain addEdge + WS reload if no context is present.
      if (ctx?.connectEdge) {
        await ctx.connectEdge(payload);
      } else {
        const { addEdge } = await import("../lib/rest");
        await addEdge(payload);
      }
    } catch (e) {
      console.error("add edge failed:", (e as Error).message);
    }
    onClose();
  };

  // If the filter narrows to a single property across all visible groups,
  // Enter creates that edge — fastest-path for keyboard users.
  const allFilteredProps = filteredGroups.flatMap((g) =>
    g.props.map((p) => ({ componentUid: g.componentUid, propUid: p.propUid })),
  );
  // Flat-index offset of each group's first prop, so one highlight index can
  // address the whole accordion. A group auto-opens when the highlight lands
  // inside it (see render), so arrowing down walks open groups for you.
  const groupPropOffsets: number[] = [];
  {
    let acc = 0;
    for (const g of filteredGroups) {
      groupPropOffsets.push(acc);
      acc += g.props.length;
    }
  }

  // "New" flow: create a component of `type` in the current folder, then connect
  // the source to its first matching-category property.
  const createNew = async (type: string) => {
    if (!ctx) return;
    // Connecting FROM an output → the new node is downstream (place it right);
    // FROM an input → the new node is upstream (place it left of the source).
    const side = sourceCategory === "output" ? "right" : "left";
    const c = await ctx.createComponent(type, { nearUid: sourceComponentUid, side });
    if (!c) {
      onClose();
      return;
    }
    const matching = Object.entries(c.properties ?? {})
      .filter(
        ([, p]) =>
          p.category === wantCategory && (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL,
      )
      .map(([name, p]) => ({ uid: p.uid, name }));
    if (matching.length === 0) {
      onClose(); // nothing connectable — leave the new node placed
    } else if (matching.length === 1) {
      await create({ componentUid: c.uid, propUid: matching[0].uid }); // one option → wire it
    } else {
      // Multiple candidates → let the user pick which prop to connect to.
      setPendingNew(c);
      setFilter("");
    }
  };
  const nf = filter.trim().toLowerCase();
  const newTypes = (ctx?.componentTypes ?? []).filter(
    (t) => !nf || t.name.toLowerCase().includes(nf) || t.type.toLowerCase().includes(nf),
  );
  // Props of the just-created component the user can pick from (pick-input mode).
  const newProps = pendingNew
    ? Object.entries(pendingNew.properties ?? {})
        .filter(
          ([, p]) =>
            p.category === wantCategory && (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL,
        )
        .map(([name, p]) => ({ uid: p.uid, name }))
    : [];
  const newPropsFiltered = nf
    ? newProps.filter((p) => p.name.toLowerCase().includes(nf))
    : newProps;

  // Position to the right of the parent menu where possible; clamp so it doesn't
  // run off-screen. The parent menu is at (x, y) and ~180px wide.
  const PICKER_W = 240;
  const left = Math.min(x + 184, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 320);

  return createPortal(
    <div
      data-ce-menu
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        left,
        top,
        zIndex: 101,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        width: PICKER_W,
        maxHeight: 320,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 11,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #2c313c" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
          {pendingNew ? (
            <>
              <button
                onClick={() => setPendingNew(null)}
                title="Back to component types"
                style={{
                  background: "transparent",
                  border: "none",
                  color: "#9ecbff",
                  cursor: "pointer",
                  fontSize: 13,
                  padding: 0,
                }}
              >
                ‹
              </button>
              <span style={{ color: "#8892a0", fontSize: 10, flex: 1 }}>
                {pendingNew.name} → pick {wantCategory === CATEGORY_INPUT ? "input" : "output"}
              </span>
            </>
          ) : creatingNew ? (
            <>
              <button
                onClick={() => setCreatingNew(false)}
                title="Back to existing components"
                style={{
                  background: "transparent",
                  border: "none",
                  color: "#9ecbff",
                  cursor: "pointer",
                  fontSize: 13,
                  padding: 0,
                }}
              >
                ‹
              </button>
              <span style={{ color: "#8892a0", fontSize: 10, flex: 1 }}>New component</span>
            </>
          ) : (
            <>
              <span style={{ color: "#8892a0", fontSize: 10, flex: 1 }}>
                Existing component → {wantCategory}…
              </span>
              {ctx && (
                <button
                  onClick={() => setCreatingNew(true)}
                  title="Create a new component and connect to it"
                  style={{
                    fontSize: 10,
                    padding: "1px 6px",
                    background: "#2c3a55",
                    color: "#9ecbff",
                    border: "1px solid #3b5388",
                    borderRadius: 3,
                    cursor: "pointer",
                    fontFamily: "inherit",
                  }}
                >
                  + New
                </button>
              )}
            </>
          )}
        </div>
        <input
          autoFocus
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              onClose();
              return;
            }
            // Tab: from pick-input, go back to the type list; otherwise toggle
            // Existing ↔ New, keeping the typed filter.
            if (e.key === "Tab") {
              e.preventDefault();
              if (pendingNew) setPendingNew(null);
              else if (ctx) setCreatingNew((v) => !v);
              return;
            }
            const len = pendingNew
              ? newPropsFiltered.length
              : creatingNew
                ? newTypes.length
                : allFilteredProps.length;
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setHighlight((h) => Math.min(h + 1, Math.max(0, len - 1)));
              return;
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setHighlight((h) => Math.max(0, h - 1));
              return;
            }
            if (e.key === "Enter") {
              e.preventDefault();
              if (pendingNew) {
                const p = newPropsFiltered[highlight];
                if (p) void create({ componentUid: pendingNew.uid, propUid: p.uid });
              } else if (creatingNew) {
                const t = newTypes[highlight];
                if (t) void createNew(t.type);
              } else {
                const p = allFilteredProps[highlight];
                if (p) void create(p);
              }
              return;
            }
            e.stopPropagation();
          }}
          placeholder={
            pendingNew ? "filter inputs…" : creatingNew ? "filter types…   ⇥ existing" : "filter…   ⇥ new"
          }
          style={{
            width: "100%",
            background: "#0f1115",
            color: "#e6e8eb",
            border: "1px solid #2c313c",
            borderRadius: 2,
            padding: "3px 6px",
            fontSize: 11,
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            boxSizing: "border-box",
            outline: "none",
          }}
        />
      </div>
      <div style={{ flex: 1, overflowY: "auto" }}>
        {pendingNew ? (
          newPropsFiltered.length === 0 ? (
            <div style={{ padding: "10px 8px", color: "#5a6172", fontSize: 11 }}>
              no matching {wantCategory === CATEGORY_INPUT ? "inputs" : "outputs"}
            </div>
          ) : (
            newPropsFiltered.map((p, i) => (
              <button
                key={p.uid}
                ref={i === highlight ? hlRef : undefined}
                onClick={() => create({ componentUid: pendingNew.uid, propUid: p.uid })}
                style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  padding: "5px 8px",
                  background: i === highlight ? "#2c3a55" : "transparent",
                  color: "#e6e8eb",
                  border: "none",
                  cursor: "pointer",
                  fontSize: 11,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#232733")}
                onMouseLeave={(e) =>
                  (e.currentTarget.style.background = i === highlight ? "#2c3a55" : "transparent")
                }
              >
                {p.name}
              </button>
            ))
          )
        ) : creatingNew ? (
          newTypes.length === 0 ? (
            <div style={{ padding: "10px 8px", color: "#5a6172", fontSize: 11 }}>
              {ctx ? "no matching types" : "unavailable"}
            </div>
          ) : (
            newTypes.map((t, i) => (
              <button
                key={t.type}
                ref={i === highlight ? hlRef : undefined}
                onClick={() => createNew(t.type)}
                style={{
                  display: "flex",
                  width: "100%",
                  textAlign: "left",
                  padding: "5px 8px",
                  background: i === highlight ? "#2c3a55" : "transparent",
                  color: "#e6e8eb",
                  border: "none",
                  cursor: "pointer",
                  fontSize: 11,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 6,
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#232733")}
                onMouseLeave={(e) =>
                  (e.currentTarget.style.background = i === highlight ? "#2c3a55" : "transparent")
                }
              >
                <span>{t.name}</span>
                <span style={{ color: "#5a6172", fontSize: 9 }}>{t.group}</span>
              </button>
            ))
          )
        ) : filteredGroups.length === 0 ? (
          <div style={{ padding: "10px 8px", color: "#5a6172", fontSize: 11 }}>
            {allComponents == null ? "loading…" : "no candidates"}
          </div>
        ) : (
          filteredGroups.map((g, idx) => {
            // Under an active filter every visible group is auto-expanded —
            // user already pre-narrowed, no need to make them click again. Also
            // auto-open the group the keyboard highlight currently sits in, so
            // arrowing down reveals props as you reach them.
            const base = groupPropOffsets[idx];
            const containsHl = highlight >= base && highlight < base + g.props.length;
            const isOpen = f ? true : expanded === g.componentUid || containsHl;
            // Section header whenever the tier changes (parent / same level /
            // inside <source> / other folders).
            const prev = idx > 0 ? filteredGroups[idx - 1] : null;
            const tier = tierOf(g);
            const showSection = tier !== (prev ? tierOf(prev) : -1);
            const sectionLabel =
              tier === 0
                ? "parent"
                : tier === 1
                  ? "same level"
                  : tier === 2
                    ? `inside ${sourceName}`
                    : "other folders";
            // Folder-chain subtitle for "other folders" rows. Drop the
            // component's own name segment, then the leading "root" so
            // root/add1 reads as /add1.
            const folderPath = g.path.replace(/\/[^/]*$/, "").replace(/^root/, "");
            const showPath = tier === 3 && folderPath !== "";
            return (
              <div key={g.componentUid}>
                {showSection && (
                  <div
                    style={{
                      padding: "6px 8px 2px 8px",
                      color: "#5a6172",
                      fontSize: 9,
                      textTransform: "uppercase",
                      letterSpacing: 0.4,
                      borderTop: idx > 0 ? "1px solid #2c313c" : "none",
                      marginTop: idx > 0 ? 2 : 0,
                    }}
                  >
                    {sectionLabel}
                  </div>
                )}
                <button
                  onClick={() =>
                    setExpanded((cur) => (cur === g.componentUid ? null : g.componentUid))
                  }
                  style={{
                    display: "flex",
                    width: "100%",
                    textAlign: "left",
                    padding: "5px 8px",
                    background: "transparent",
                    color: "#e6e8eb",
                    border: "none",
                    cursor: "pointer",
                    fontSize: 11,
                    fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    alignItems: "center",
                    gap: 6,
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "#232733")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                >
                  <span style={{ color: "#8892a0", width: 8, flexShrink: 0 }}>
                    {isOpen ? "▾" : "▸"}
                  </span>
                  <span
                    style={{
                      flex: 1,
                      minWidth: 0,
                      display: "flex",
                      flexDirection: "column",
                      overflow: "hidden",
                    }}
                  >
                    <span
                      style={{
                        color: "#9ecbff",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {g.componentName}
                      {g.isParent && (
                        <span
                          style={{
                            marginLeft: 6,
                            fontSize: 8,
                            textTransform: "uppercase",
                            letterSpacing: 0.4,
                            color: "#ffd479",
                            border: "1px solid #5a4a2a",
                            background: "#2a2418",
                            borderRadius: 3,
                            padding: "0 4px",
                          }}
                        >
                          parent
                        </span>
                      )}
                    </span>
                    {showPath && (
                      <span
                        style={{
                          color: "#5a6172",
                          fontSize: 9,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                        title={g.path}
                      >
                        {folderPath}
                      </span>
                    )}
                  </span>
                  <span style={{ color: "#5a6172", fontSize: 10 }}>{g.props.length}</span>
                </button>
                {isOpen && (
                  <div style={{ paddingBottom: 2 }}>
                    {g.props.map((p, pi) => {
                      const isHl = base + pi === highlight;
                      return (
                        <button
                          key={p.propUid}
                          ref={isHl ? hlRef : undefined}
                          onClick={() =>
                            create({ componentUid: g.componentUid, propUid: p.propUid })
                          }
                          style={{
                            display: "block",
                            width: "100%",
                            textAlign: "left",
                            padding: "3px 8px 3px 28px",
                            background: isHl ? "#2c3a55" : "transparent",
                            color: "#e6e8eb",
                            border: "none",
                            cursor: "pointer",
                            fontSize: 11,
                            fontFamily: "ui-monospace, SFMono-Regular, monospace",
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.background = "#2c313c")}
                          onMouseLeave={(e) =>
                            (e.currentTarget.style.background = isHl ? "#2c3a55" : "transparent")
                          }
                        >
                          {p.propName}
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>,
    document.body,
  );
}

function MenuItem({ onClick, label, danger }: { onClick: () => void; label: string; danger?: boolean }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "block",
        width: "100%",
        textAlign: "left",
        padding: "5px 8px",
        background: "transparent",
        color: danger ? "#ffb8b8" : "#e6e8eb",
        border: "none",
        borderRadius: 2,
        cursor: "pointer",
        fontSize: 11,
        fontFamily: "inherit",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.background = "#2c313c")}
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
    >
      {label}
    </button>
  );
}

// Inline value editor for inputs / config properties. Click → text input opens with
// the current value selected. Enter or blur → PATCH /nodes/uid/{uid}. Escape → cancel.
//
// Why direct PATCH rather than going through a parent callback: keeps the editor
// self-contained, no prop drilling, and the topology event the engine fires in
// response refreshes the live value via the normal value-plane path.
function PropertyValueEditor({
  componentUid,
  propName,
  value,
  dataType,
  facet,
}: {
  componentUid: number;
  propName: string;
  value: DecodedValue | undefined;
  dataType: PropertyDataType;
  facet?: PropFacet;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<string>("");

  const display = fmtValueFacet(value, dataType, facet);
  const start = () => {
    setDraft(value == null ? "" : typeof value === "string" ? value : String(value));
    setEditing(true);
  };
  const commit = async () => {
    setEditing(false);
    const raw = draft.trim();
    if (raw === "") return;
    // Parse the draft according to the property's dataType. Strings go through as-is.
    let parsed: string | number | boolean;
    if (dataType === DATATYPE_BOOL) {
      const lower = raw.toLowerCase();
      parsed = lower === "true" || lower === "1" || lower === "yes";
    } else if (dataType === DATATYPE_STRING) {
      parsed = raw;
    } else {
      const n = Number(raw);
      if (!Number.isFinite(n)) return;
      parsed = n;
    }
    try {
      const { updateNode } = await import("../lib/rest");
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", (e as Error).message);
    }
  };

  if (editing) {
    const stop = (e: React.SyntheticEvent) => e.stopPropagation();
    // Aliased value (bool or int enum) → a dropdown of the alias labels, writing
    // back the native value (the code; bool → code 1/0).
    if (facet?.aliases && facet.aliases.length) {
      const cur =
        value === true ? 1 : value === false ? 0 : typeof value === "number" ? value : Number(value);
      return (
        <select
          autoFocus
          className="nodrag"
          value={String(cur)}
          onChange={(e) => commitAlias(Number(e.target.value))}
          onKeyDown={(e) => {
            if (e.key === "Escape") setEditing(false);
            e.stopPropagation();
          }}
          onBlur={() => setEditing(false)}
          onClick={stop}
          onPointerDown={stop}
          style={editorInputStyle}
        >
          {facet.aliases.map((a) => (
            <option key={a.code} value={String(a.code)}>
              {a.label}
            </option>
          ))}
        </select>
      );
    }
    // `nodrag` is React Flow's opt-out class: nodes won't start a drag from
    // pointer events on elements carrying it. Critical for native form
    // controls (especially <select>) because the OS dropdown captures the
    // pointer events — RF sees pointerdown but never the pointerup, leaving
    // its drag state stuck and the node ends up following the cursor after
    // the user picks an option.
    if (dataType === DATATYPE_BOOL) {
      return (
        <select
          autoFocus
          className="nodrag"
          value={draft}
          onChange={(e) => commitWith(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setEditing(false);
            e.stopPropagation();
          }}
          onBlur={() => setEditing(false)}
          onClick={stop}
          onPointerDown={stop}
          style={editorInputStyle}
        >
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      );
    }
    return (
      <input
        autoFocus
        className="nodrag"
        type={dataType === DATATYPE_NUMBER ? "number" : "text"}
        // Sensible step for the number spinner — full integer step by default,
        // user can still type any decimal. inputMode keeps mobile keyboards
        // sane.
        inputMode={dataType === DATATYPE_NUMBER ? "decimal" : undefined}
        step={dataType === DATATYPE_NUMBER ? "any" : undefined}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          else if (e.key === "Escape") setEditing(false);
          e.stopPropagation();
        }}
        onBlur={commit}
        onClick={stop}
        onPointerDown={stop}
        style={editorInputStyle}
      />
    );
  }
  return (
    <span
      // No `nodrag` / pointerdown-stop here: a press-and-drag starting on a
      // value should move the node like any other body grab. A plain click
      // (movement < nodeDragThreshold) still falls through to onClick → edit.
      onClick={(e) => {
        e.stopPropagation();
        start();
      }}
      style={{
        color: dataType === DATATYPE_BOOL ? COLOR_BOOL : "#e6e8eb",
        fontVariantNumeric: "tabular-nums",
        cursor: "text",
        padding: "0 2px",
        borderRadius: 2,
      }}
      title="click to edit"
    >
      {display}
    </span>
  );

  // Inline helper that commits a specific raw value (used by the bool select
  // since onChange fires with the new value before setDraft would land).
  async function commitWith(raw: string) {
    setDraft(raw);
    setEditing(false);
    let parsed: string | number | boolean;
    if (dataType === DATATYPE_BOOL) {
      parsed = raw === "true";
    } else if (dataType === DATATYPE_STRING) {
      parsed = raw;
    } else {
      const n = Number(raw);
      if (!Number.isFinite(n)) return;
      parsed = n;
    }
    try {
      const { updateNode } = await import("../lib/rest");
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", (e as Error).message);
    }
  }

  // Commit an aliased selection: write the native value (bool → code 1/0,
  // otherwise the int code itself).
  async function commitAlias(code: number) {
    setEditing(false);
    const parsed: number | boolean = dataType === DATATYPE_BOOL ? code === 1 : code;
    try {
      const { updateNode } = await import("../lib/rest");
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", (e as Error).message);
    }
  }
}

const editorInputStyle: React.CSSProperties = {
  width: 90,
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #4a9eff",
  borderRadius: 2,
  padding: "0 4px",
  fontFamily: "inherit",
  fontSize: 11,
  textAlign: "right",
  outline: "none",
};

const overrideInputStyle: React.CSSProperties = {
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #4a9eff",
  borderRadius: 2,
  padding: "3px 6px",
  fontFamily: "ui-monospace, SFMono-Regular, monospace",
  fontSize: 11,
  outline: "none",
};

function StatusDot({ color, text }: { color: string; text: string }) {
  const [hover, setHover] = useState(false);
  return (
    <span
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
      }}
    >
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: color,
          boxShadow: "0 0 0 1px rgba(0,0,0,0.4)",
          display: "block",
        }}
      />
      {hover && (
        <span
          style={{
            position: "absolute",
            top: "100%",
            right: 0,
            marginTop: 6,
            padding: "3px 7px",
            background: "#0f1115",
            border: "1px solid " + color,
            borderRadius: 3,
            color: "#e6e8eb",
            fontSize: 10,
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            whiteSpace: "nowrap",
            zIndex: 50,
            pointerEvents: "none",
            boxShadow: "0 2px 6px rgba(0,0,0,0.5)",
          }}
        >
          {text || "—"}
        </span>
      )}
    </span>
  );
}

// The engine emits the status property's VALUE as a JSON object serialized to
// a string. An empty object "{}" means the component is healthy. A populated
// object carries fields like { error: "...", warning: "..." } — for now we
// surface the first non-empty string field as a hint. Logged as API_GAPS #8
// because the encoding is ambiguous: it's a string-shaped object, the client
// has to JSON.parse to know whether to render "ok" or an error label.
function parseStatus(raw: unknown): string {
  if (raw == null) return "";
  if (typeof raw !== "string") {
    // Engine could conceivably stop double-encoding one day — accept a real
    // object too without bothering the parser.
    if (typeof raw === "object") return summarizeStatusObject(raw);
    return String(raw);
  }
  // Strings that match a known label (engine might switch to plain labels
  // later) are returned as-is.
  const t = raw.trim();
  if (t === "" || t === "{}") return "";
  try {
    const obj = JSON.parse(t);
    if (obj == null) return "";
    if (typeof obj === "string") return obj;
    if (typeof obj === "object") return summarizeStatusObject(obj);
  } catch {
    // Not JSON — treat as plain label.
  }
  return t;
}

function summarizeStatusObject(obj: object): string {
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === "string" && v.trim() !== "") return `${k}: ${v}`;
    if (typeof v === "boolean" && v) return k;
    if (typeof v === "number" && v !== 0) return `${k}=${v}`;
  }
  return "";
}

// Map a status value to an indicator color. Unknown / empty / "NONE" reads as healthy.
// Any non-empty value the engine emits that isn't recognised falls into the "other"
// bucket and shows a neutral grey — better than nothing while we learn the vocabulary.
function statusColorFor(s: string): { bg: string; label: string } {
  const v = s.toUpperCase();
  if (!v || v === "NONE" || v === "OK") return { bg: "#4ade80", label: v || "ok" };
  if (v === "STALE") return { bg: "#f59e0b", label: "stale" };
  if (v === "OVERRIDDEN") return { bg: "#9ecbff", label: "overridden" };
  if (v === "ERROR" || v === "FAULT" || v === "DOWN") return { bg: "#ef4444", label: v.toLowerCase() };
  return { bg: "#8892a0", label: s };
}

interface InnerProps {
  data: FunctionBlockData;
  // React Flow passes `selected` to every node component. We use it to paint the
  // selection highlight ourselves, since custom node types don't inherit the default
  // RF .selected outline.
  selected?: boolean;
}

function FunctionBlockInner({ data, selected }: InnerProps) {
  // ALL hooks run unconditionally before any early-return branch — otherwise React
  // loses its hook-order invariant and throws "Rendered more hooks than during the
  // previous render."
  const schemaV = useSchemaVersion((s) => s.version);
  const ctx = useContext(CeWiresheetContext);
  // Subscribe to the live REST component so structural changes re-render the
  // block without a manual reload. Default Object.is equality only re-renders when
  // THIS uid's entry changes (upsertComponent swaps just that one entry).
  const restComp = useStructural((s) => s.components.get(data.componentUid));
  // The uids of THIS component's properties. Pre-computed so the per-component
  // selectors below don't have to walk restComp.properties on every state
  // notification.
  const ourUids = useMemo(() => {
    if (!restComp) return [] as number[];
    const own = Object.values(restComp.properties).map((p) => p.uid);
    // Also subscribe to the values of any child props this component exposes as
    // ports — they're off-canvas, so they wouldn't otherwise stream to us.
    for (const ep of exposedPorts(facetFor(restComp.uid, rawFacet(restComp.properties)))) {
      own.push(ep.childUid); // the port's live value
      if (ep.facet.facetProp != null) own.push(ep.facet.facetProp); // child's live __facets
    }
    return own;
  }, [restComp]);
  // Level-of-detail: true when zoomed out far enough that values aren't legible.
  // Boolean selector → this node only re-renders when CROSSING the threshold,
  // not on every zoom delta.
  const lod = useRfStore((s) => s.transform[2] < LOD_ZOOM);
  // Per-component value subscription. At scale (many components in the view),
  // subscribing to the global `version` counter forces every FunctionBlock to
  // re-render on every WS frame — that's the source of the "view feels frozen"
  // pain when you have 100+ nodes. Shallow-equality selector means a frame
  // touching unrelated uids returns the same Record and Zustand skips us.
  // In LOD we return a stable empty object so value changes don't re-render a
  // node that isn't drawing values.
  const valuesByUid = useValues(
    useShallow((s) => {
      if (lod) return EMPTY_VALUES;
      const out: Record<number, DecodedValue | undefined> = {};
      for (const uid of ourUids) out[uid] = s.values.get(uid);
      return out;
    }),
  );
  const flagsByUid = useStatusFlags(
    useShallow((s) => {
      if (lod) return EMPTY_FLAGS;
      const out: Record<number, number> = {};
      for (const uid of ourUids) out[uid] = s.flags.get(uid) ?? 0;
      return out;
    }),
  );
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    propName: string;
    propUid: number;
    category: PropertyCategory;
    dataType: PropertyDataType;
    currentValue: DecodedValue | undefined;
    overridden: boolean;
    exposed?: boolean;
    exposedComponent?: number;
    portOwner?: number;
  } | null>(null);
  void schemaV;

  // Collaborators (other sessions) who currently have THIS component selected.
  // Per-component selector with shallow equality. CRITICAL: the selector must
  // return PRIMITIVES, not fresh objects. useShallow compares array elements
  // with Object.is — new {name,color} objects every call never match, so it
  // returns a new array every render → "getSnapshot should be cached" infinite
  // loop (blanks the tree). Encoding each as a "color\tname" string makes the
  // shallow compare value-based and stable; we split in render.
  const otherSelectorKeys = usePresence(
    useShallow((s) => {
      const out: string[] = [];
      for (const c of s.collaborators.values()) {
        if (c.state.selectedComponents?.includes(data.componentUid)) {
          const name = c.state.userName ?? c.sessionId.slice(0, 6);
          out.push(`${PRESENCE_PALETTE[c.colorIdx]}\t${name}`);
        }
      }
      return out;
    }),
  );
  const otherSelectors = otherSelectorKeys.map((k) => {
    const [color, name] = k.split("\t");
    return { color, name };
  });

  // Structural derivation (rows, node height, status indicator) — pure function
  // of the REST component + the WS schema. Memoized so the per-FRAME value/
  // status re-renders (chatty math nodes re-render ~10×/s) DON'T rebuild the
  // row list, re-filter by category, or re-scan for the status prop every time.
  // Only a real structural change (props added/removed → restComp identity
  // swaps) or a schema arrival (dataType table fills → schemaV bumps) recomputes
  // this. The live value/flag reads stay in the row JSX below.
  // Cross-session facet sync, done SAFELY: the structural memo below renders from
  // the REST copy of __facets (authoritative — never clobbered by a stale/empty
  // stream value, which was breaking expose). The live stream is used ONLY as a
  // "something changed" trigger: __facets is an input string, so its value streams
  // into our value map keyed by its own uid; when that streamed value TRANSITIONS
  // (another session edited the facet), request a debounced scope reload so REST
  // refreshes structural and the rows/ports/edges rebuild consistently. Compared
  // against the previous STREAMED value (not structural) so it can't loop.
  const ownFacetUid = restComp?.properties[FACET_PROP]?.uid;
  const liveFacetRaw =
    ownFacetUid != null && typeof valuesByUid[ownFacetUid] === "string"
      ? (valuesByUid[ownFacetUid] as string)
      : undefined;
  const prevFacetRaw = useRef<string | null>(null);
  useEffect(() => {
    if (liveFacetRaw == null) return;
    if (prevFacetRaw.current === null) {
      prevFacetRaw.current = liveFacetRaw; // seed; don't fire on first sight
      return;
    }
    if (liveFacetRaw !== prevFacetRaw.current) {
      prevFacetRaw.current = liveFacetRaw;
      ctx?.requestReload?.();
    }
  }, [liveFacetRaw, ctx]);

  const structural = useMemo(() => {
    if (!restComp) return null;
    // User-facing = normal role (the `system` bool is gone; systemRole != 0
    // means an engine-managed slot).
    const isUserFacing = (p: Property) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL;
    const entries = Object.entries(restComp.properties);
    // Parse this component's __facet (cached by raw string) and attach each
    // prop's metadata to its row. Hidden rows are dropped; `order` sorts within
    // each category group (stable for rows without it).
    // Authoritative: the REST copy of __facets (cached by raw string). The live
    // stream only triggers a reload (above) — it never sources the render, so a
    // stale/empty streamed value can't blank out exposed ports.
    const facet = facetFor(restComp.uid, rawFacet(restComp.properties));
    const mappedRows: PropRow[] = entries
      .filter(([, p]) => isUserFacing(p))
      .map(([name, p]) => ({
        uid: p.uid,
        name,
        category: p.category,
        dataType: propertyDataType.get(p.uid) ?? inferDataType(p.value),
        systemRole: p.systemRole,
        facet: facet.get(p.uid),
      }));
    const hiddenCount = mappedRows.filter((r) => r.facet?.hidden).length;
    const userRows = mappedRows.filter((r) => !r.facet?.hidden);
    // Exposed ports: child props this component projects as its own input/output
    // ports (see FACET_DESIGN.md §9). uid = the child prop uid (its handle id),
    // dataType from the global schema index, value via the subscription above.
    // Read-only here — you edit the real value inside the child.
    const portRows: PropRow[] = exposedPorts(facet).map((ep) => ({
      uid: ep.childUid,
      name: ep.facet.label ?? `#${ep.childUid}`,
      category: ep.side === "input" ? CATEGORY_INPUT : CATEGORY_OUTPUT,
      dataType: propertyDataType.get(ep.childUid) ?? inferDataType(undefined),
      facet: ep.facet,
      exposed: true,
      exposedComponent: ep.facet.childComponent,
      facetPropUid: ep.facet.facetProp,
    }));
    const allRows = [...userRows, ...portRows];
    const byOrder = (a: PropRow, b: PropRow) =>
      (a.facet?.order ?? Number.MAX_SAFE_INTEGER) - (b.facet?.order ?? Number.MAX_SAFE_INTEGER);
    const rows: PropRow[] = [
      ...allRows.filter((r) => r.category === CATEGORY_OUTPUT).sort(byOrder),
      ...allRows.filter((r) => r.category === CATEGORY_INPUT).sort(byOrder),
      ...allRows.filter((r) => r.category === CATEGORY_CONFIG).sort(byOrder),
    ];
    const statusEntry = entries.find(([, p]) => p.systemRole === ROLE_STATUS);
    const statusText = parseStatus(statusEntry?.[1].value);
    return {
      rows,
      // + ROW_H for the bottom lip (drill-in button + action marker).
      nodeH: TITLE_H + rows.length * ROW_H + ROW_H,
      kind: restComp.type,
      statusText,
      statusColor: statusColorFor(statusText),
      statusPropExists: statusEntry != null,
      hiddenCount,
    };
    // schemaV in deps: when the WS schema fills propertyDataType, recompute
    // dataTypes. restComp identity swaps on any structural change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [restComp, schemaV]);

  // Count this render for diagnostics. A re-render storm (every node
  // re-rendering on every frame) shows up as renders/sec ≈ frames/sec ×
  // node-count in the DiagPanel.
  diagRecordRender("FunctionBlock");

  if (!restComp || !structural) {
    // REST hasn't landed yet — render a placeholder. As soon as `components`
    // populates this uid, the Zustand selector re-renders us.
    return (
      <div
        style={{
          width: NODE_W,
          height: 40,
          background: "#1a1d24",
          border: "1px dashed #3b4350",
          borderRadius: 4,
          color: "#8892a0",
          fontSize: 11,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
        }}
      >
        uid {data.componentUid}
      </div>
    );
  }
  // Live per-component value / status maps (re-read each render; the row JSX
  // below reads these by uid). The structural shell (rows, nodeH, status text)
  // comes from the memo above and is NOT rebuilt on a value-only re-render.
  const values = valuesByUid;
  const statusFlagsMap = flagsByUid;
  const { rows, nodeH, kind, statusText, statusColor, statusPropExists, hiddenCount } = structural;

  return (
    <div
      onContextMenu={(e) => {
        // Resolve the row by walking up from the actual click target until we
        // hit an element tagged with data-row-uid. Both row divs AND their
        // Handle siblings carry the attribute, so a right-click anywhere on
        // the row (label, value, handle hit zone) lands on the same row.
        // Title bar has its own onContextMenu that stopPropagation()s, so
        // this only fires for body clicks.
        let el = e.target as Element | null;
        let uid: number | null = null;
        while (el && el !== e.currentTarget) {
          const v = (el as HTMLElement).dataset?.rowUid;
          if (v != null) {
            uid = Number(v);
            break;
          }
          el = el.parentElement;
        }
        if (uid == null) return;
        const p = rows.find((r) => r.uid === uid);
        if (!p) return;
        e.preventDefault();
        e.stopPropagation();
        const flags = statusFlagsMap[p.uid] ?? restComp.properties[p.name]?.statusFlags ?? 0;
        setMenu({
          x: e.clientX,
          y: e.clientY,
          propName: p.name,
          propUid: p.uid,
          category: p.category,
          dataType: p.dataType,
          currentValue: values[p.uid],
          overridden: (flags & STATUS_OVERRIDDEN) !== 0,
          exposed: !!p.exposed,
          exposedComponent: p.exposedComponent,
          portOwner: p.exposed ? data.componentUid : undefined,
        });
      }}
      style={{
        width: NODE_W,
        minHeight: nodeH,
        background: "#1a1d24",
        border: selected
          ? "1px solid #4a9eff"
          : otherSelectors.length > 0
            ? `1px solid ${otherSelectors[0].color}`
            : "1px solid #2c313c",
        borderRadius: 4,
        color: "#e6e8eb",
        fontSize: 11,
        // Selection glow priority: our own selection (blue) wins; otherwise a
        // collaborator's selection paints a glow in their color. Both stack
        // their shadow over the default drop shadow.
        boxShadow: selected
          ? "0 0 0 1px #4a9eff, 0 0 12px rgba(74,158,255,0.45)"
          : otherSelectors.length > 0
            ? `0 0 0 1px ${otherSelectors[0].color}, 0 0 10px ${otherSelectors[0].color}66`
            : "0 1px 2px rgba(0,0,0,0.4)",
        transition: "box-shadow 80ms ease, border-color 80ms ease",
        position: "relative",
        overflow: "visible",
      }}
    >
      {otherSelectors.length > 0 && (
        <div
          style={{
            position: "absolute",
            top: -9,
            left: 6,
            display: "flex",
            gap: 3,
            zIndex: 5,
            pointerEvents: "none",
          }}
        >
          {otherSelectors.map((o) => (
            <span
              key={o.name}
              title={`${o.name} has this selected`}
              style={{
                fontSize: 9,
                lineHeight: "12px",
                padding: "0 4px",
                background: o.color,
                color: "#0f1115",
                borderRadius: 2,
                fontWeight: 600,
                fontFamily: "ui-monospace, SFMono-Regular, monospace",
                whiteSpace: "nowrap",
              }}
            >
              {o.name}
            </span>
          ))}
        </div>
      )}
      <div
        onContextMenu={(e) => {
          // Node-level menu fires only when the user right-clicked the TITLE
          // BAR. The body below has its own per-row context menus (property
          // overrides / Connect to…) and right-clicking blank space between
          // rows should do nothing — keeps the body's right-click reserved
          // for property-targeted actions.
          if (!data.onContextMenu) return;
          e.preventDefault();
          e.stopPropagation();
          data.onContextMenu(data.componentUid, e.clientX, e.clientY);
        }}
        // Double-click the title to drill into the component's level (every
        // component can contain children, even if empty). Only here + the lip,
        // not the value rows (which use single-click to edit).
        onDoubleClick={(e) => {
          e.stopPropagation();
          data.onEnter?.(data.componentUid);
        }}
        style={{
          height: TITLE_H,
          padding: "4px 8px",
          background: "#232733",
          borderBottom: "1px solid #2c313c",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          boxSizing: "border-box",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span
            style={{
              fontWeight: 600,
              fontSize: 12,
              flex: 1,
              minWidth: 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.name ?? kind}
          </span>
          {statusPropExists && <StatusDot color={statusColor.bg} text={statusText} />}
        </div>
        <div
          style={{
            fontSize: 10,
            lineHeight: 1.35,
            color: "#8892a0",
            fontFamily: "ui-monospace, monospace",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
          title={kind}
        >
          {kind}
        </div>
      </div>

      {/* Bottom lip — a prop-height footer below the last row. Holds the
          drill-in (↵) button when this component has children, and a ⚡ marker
          when its type has actions. Hidden in LOD like the rows. */}
      {!lod && (
        <div
          onDoubleClick={(e) => {
            e.stopPropagation();
            data.onEnter?.(data.componentUid);
          }}
          title="Double-click to enter this component's level"
          style={{
            position: "absolute",
            left: 0,
            right: 0,
            top: TITLE_H + rows.length * ROW_H,
            height: ROW_H,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "0 8px",
            boxSizing: "border-box",
            borderTop: "1px solid #2c313c",
            background: "#1e222b",
            borderBottomLeftRadius: 4,
            borderBottomRightRadius: 4,
            cursor: "pointer",
          }}
        >
          {/* left: actions marker + hidden-props indicator */}
          <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span
              title={data.hasActions ? "This component has actions" : undefined}
              style={{ fontSize: 11, color: "#ffd166", lineHeight: `${ROW_H}px` }}
            >
              {data.hasActions ? "⚡" : ""}
            </span>
            {hiddenCount > 0 && (
              <span
                title={`${hiddenCount} hidden propert${hiddenCount === 1 ? "y" : "ies"}`}
                style={{ fontSize: 11, color: "#5a6172", lineHeight: `${ROW_H}px` }}
              >
                ⊘
              </span>
            )}
          </span>
          {/* right: has-children marker (double-click the block to enter) */}
          {data.hasChildren && (
            <span
              title={`Has ${data.childCount ?? ""} child${
                data.childCount === 1 ? "" : "ren"
              } — double-click to enter`}
              style={{
                fontSize: 11,
                color: "#9ecbff",
                lineHeight: `${ROW_H}px`,
                fontFamily: "ui-monospace, SFMono-Regular, monospace",
              }}
            >
              ⧉ {data.childCount ?? ""}
            </span>
          )}
        </div>
      )}

      {/* Row CONTENT (labels + value cells) only at normal zoom. In LOD the
          node is just its title bar + handles — values aren't legible anyway,
          and skipping these divs is the bulk of the zoomed-out render saving. */}
      {!lod && rows.map((p, i) => {
        const isInput = p.category === CATEGORY_INPUT;
        const isOutput = p.category === CATEGORY_OUTPUT;
        const v = values[p.uid];
        // An exposed port reads its presentation LIVE from the child's streamed
        // __facets (no stale copy). Live label/unit/aliases win; the baked label
        // is just a fallback name.
        let rowFacet = p.facet;
        if (p.exposed && p.facetPropUid != null && p.exposedComponent != null) {
          const fv = values[p.facetPropUid];
          if (typeof fv === "string") {
            const live = facetFor(p.exposedComponent, fv).get(p.uid);
            if (live) rowFacet = { ...p.facet, ...live, label: live.label ?? p.facet?.label };
          }
        }
        // Status: prefer the live WS-driven map (updated by STATUS sections in
        // every binary frame), fall back to the REST snapshot.
        const flags = statusFlagsMap[p.uid] ?? restComp.properties[p.name]?.statusFlags ?? 0;
        const overridden = (flags & STATUS_OVERRIDDEN) !== 0;
        // `editable` here gates the inline click-to-edit. Inputs and config
        // properties PATCH /nodes which sets the value directly. Outputs are
        // engine-computed — PATCH /nodes wouldn't take, but PATCH /overrides
        // still freezes them. So outputs aren't inline-editable here but ARE
        // overridable via the right-click menu (which uses /overrides).
        // Exposed ports are read-only here (edit the real value inside the
        // child); their value still displays + the handle is wired.
        const editable = !p.exposed && (isInput || p.category === CATEGORY_CONFIG);
        // Hover tooltip exposes the real uids for debugging — for an exposed port
        // that's the CHILD's prop + component, not the folder's.
        const rowTitle = `${p.name} — prop uid ${p.uid} · component uid ${
          p.exposed ? (p.exposedComponent ?? "?") : data.componentUid
        }`;
        return (
          <div
            key={p.uid}
            data-row-uid={p.uid}
            title={rowTitle}
            // NOTE: no `nodrag` here. The row must be draggable so the user can
            // grab the node anywhere on its body, not just the 40px title bar.
            // Only the genuinely interactive controls carry `nodrag` — the
            // inline editor input/select while editing (so text-select / the
            // dropdown work). A plain click on a value still edits (RF's
            // nodeDragThreshold treats <4px as a click, not a drag).
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              top: TITLE_H + i * ROW_H,
              height: ROW_H,
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              padding: "0 12px",
              fontSize: 11,
              fontFamily: "ui-monospace, SFMono-Regular, monospace",
              background: overridden ? "rgba(245,158,11,0.08)" : "transparent",
            }}
          >
            <span
              style={{
                color: isInput ? "#8892a0" : isOutput ? "#cbd3e0" : "#9aa3b2",
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              {p.exposed && (
                <span style={{ color: "#7a8a9f" }} title="exposed from a child">
                  ↪
                </span>
              )}
              <span title={rowFacet?.label ? p.name : undefined}>{rowFacet?.label ?? p.name}</span>
              {p.category === CATEGORY_CONFIG ? " (cfg)" : ""}
              {overridden && (
                <span
                  title="overridden"
                  style={{
                    fontSize: 9,
                    padding: "0 4px",
                    background: "#f59e0b",
                    color: "#0f1115",
                    borderRadius: 2,
                    fontWeight: 600,
                  }}
                >
                  OVR
                </span>
              )}
            </span>
            {editable ? (
              <PropertyValueEditor
                componentUid={data.componentUid}
                propName={p.name}
                value={v}
                dataType={p.dataType}
                facet={rowFacet}
              />
            ) : (
              <span
                style={{
                  color: p.dataType === DATATYPE_BOOL ? COLOR_BOOL : "#e6e8eb",
                  fontVariantNumeric: "tabular-nums",
                  // Same 2px horizontal padding as the inline editor's display
                  // span, so input and output values line up at the same right
                  // edge instead of outputs sitting 2px further right.
                  padding: "0 2px",
                }}
                title={DATATYPE_LABEL[p.dataType]}
              >
                {fmtValueFacet(v, p.dataType, rowFacet)}
              </span>
            )}
          </div>
        );
      })}

      {rows.map((p, i) => {
        if (p.category === CATEGORY_CONFIG) return null;
        const isInput = p.category === CATEGORY_INPUT;
        const c = colorForType(p.dataType);
        // The Handle div is an invisible hit zone — full row height, ~35px wide,
        // anchored flush against the port-side edge of the node and extending INWARD.
        // The small 8x8 colored marker (rendered as a child) sits at the port edge so
        // the visible affordance and the connection anchor both align with the node
        // boundary. React Flow uses the handle's position prop (Left/Right) to pick
        // which edge of the bounding rect to anchor edges at, so geometry stays clean.
        const HANDLE_W = 35;
        const rowTop = TITLE_H + i * ROW_H;
        return (
          <Handle
            key={`h-${p.uid}`}
            id={String(p.uid)}
            type={isInput ? "target" : "source"}
            position={isInput ? Position.Left : Position.Right}
            // Same data-row-uid attribute as the row div — the root's
            // onContextMenu walks up from e.target to find this, so
            // right-clicks land on the right row regardless of whether
            // the cursor was over the row div, label, value, or the
            // handle hit zone overlay.
            data-row-uid={p.uid}
            style={{
              top: rowTop,
              [isInput ? "left" : "right"]: 0,
              width: HANDLE_W,
              height: ROW_H,
              background: "transparent",
              border: "none",
              borderRadius: 0,
              // Cancel React Flow's default translate (which would push the handle
              // outside the node by 50% of its width). With translate(0,0) the box
              // sits exactly where `left:0` / `right:0` puts it — flush at the edge.
              transform: "none",
            }}
          >
            <span
              style={{
                position: "absolute",
                top: "50%",
                // Center the visible marker ON the port-side edge of the hit box:
                //   input → x=0 (box's left edge = node's left edge)
                //   output → x=100% (box's right edge = node's right edge)
                left: isInput ? 0 : "100%",
                transform: "translate(-50%, -50%)",
                width: 8,
                height: 8,
                background: c,
                border: "1px solid #0f1115",
                borderRadius: 1,
                pointerEvents: "none",
              }}
            />
          </Handle>
        );
      })}
      {menu && (
        <PropertyContextMenu
          x={menu.x}
          y={menu.y}
          propName={menu.propName}
          propUid={menu.propUid}
          category={menu.category}
          dataType={menu.dataType}
          currentValue={menu.currentValue}
          overridden={menu.overridden}
          exposed={menu.exposed}
          portOwner={menu.portOwner}
          componentUid={menu.exposedComponent ?? data.componentUid}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  );
}

export const FunctionBlock = memo(FunctionBlockInner, (a, b) => {
  return (
    a.selected === b.selected &&
    a.data.componentUid === b.data.componentUid &&
    a.data.name === b.data.name &&
    a.data.hasChildren === b.data.hasChildren &&
    a.data.childCount === b.data.childCount &&
    a.data.onEnter === b.data.onEnter &&
    a.data.onContextMenu === b.data.onContextMenu
  );
});

// --- Ghost node ---
// A sub-node placeholder for the off-canvas endpoint of a cross-folder edge.
// One row tall (lines up flush with the connected property on the real
// component), shows the external component's path + prop name, double-click
// jumps the breadcrumb to that component's folder.
//
// Has exactly one handle (`target` if it represents an external INPUT being
// fed by a visible output; `source` if it represents an external OUTPUT
// feeding a visible input). The handle id is the external property's uid so
// the cross-folder edge connects cleanly through React Flow's normal handle
// routing.

export interface GhostConnection {
  externalComponentUid: number;
  externalPath: string;     // e.g. "root/Services/foo"
  externalPropName: string;
  // Edge uid backing this connection — lets the popover delete a specific
  // edge from a fan-out without having to disambiguate via paths.
  edgeUid: number;
}

export type GhostNodeData = {
  // One ghost represents ONE handle on the visible component (one row on the
  // visible component) and aggregates ALL cross-folder edges that share that
  // endpoint. An input ghost has exactly one connection (inputs take at most
  // one incoming edge); an output ghost can have many (outputs fan out).
  // Without aggregation, multiple output-side ghosts would render at the
  // same Y and visually overlap into a single illegible blob.
  connections: GhostConnection[];
  // Shared handle id. All cross-folder edges that share this ghost reference
  // this id as source/targetHandle so they all converge on the same point.
  handleId: string;
  // Which side carries the handle.
  //   "input"  → ghost is the TARGET of an edge from a visible output, so its
  //              handle is on the LEFT (incoming). Connections list the
  //              external INPUT(s) being fed.
  //   "output" → ghost is the SOURCE of an edge into a visible input, so its
  //              handle is on the RIGHT (outgoing). Connections list the
  //              external OUTPUT(s) feeding it (almost always just one, but
  //              we use the same shape for symmetry).
  side: "input" | "output";
  // The visible component this ghost is anchored to, plus the row index of
  // the connected property — together these let App.tsx recompute the
  // ghost's position when the anchor component is dragged, so the ghost
  // follows along instead of being left behind.
  anchorUid: number;
  anchorRowIdx: number;
  // Width of THIS specific ghost, sized to its content by ghostWidthFor().
  // Stored on the data so the drag-along recomputation in App.tsx can place
  // left-side ghosts (output-source case) flush against the anchor's left
  // edge without recomputing the text length.
  width: number;
  // Navigate to a specific external component's folder (push crumbs).
  onNavigate?: (uid: number) => void;
  // Delete a specific edge backing one of this ghost's connections. The
  // ghost auto-disappears once its connections list empties — App.tsx
  // removes the ghost node when the connection count hits zero.
  onDeleteEdge?: (edgeUid: number) => void | Promise<void>;
};

function GhostNodeInner({ data }: { data: GhostNodeData }) {
  const isInputSide = data.side === "input";
  const [popOpen, setPopOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  const count = data.connections.length;
  // The label we show in the collapsed pill. Path shows the WHOLE path
  // including the component's own name (so "Services/RestService", not just
  // the folder "Services"), with the leading "root/" stripped — every path
  // starts with root, so the prefix is noise.
  const first = data.connections[0];
  const labelLeft = stripRoot(first?.externalPath ?? "");
  const labelRight = first?.externalPropName ?? "";

  const onClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    // Always open the popover. For N=1 it's a single-row "click to navigate"
    // affordance; for N>1 it lists all targets. Either way the interaction
    // is uniform — no special-case double-click semantics to remember.
    setPopOpen((v) => !v);
  };

  // Dismiss popover on outside click.
  useEffect(() => {
    if (!popOpen) return;
    const dismiss = (ev: MouseEvent) => {
      const el = ev.target as Element | null;
      if (el && el.closest("[data-ce-ghost-pop]")) return;
      if (el && rootRef.current?.contains(el)) return;
      setPopOpen(false);
    };
    document.addEventListener("mousedown", dismiss);
    return () => document.removeEventListener("mousedown", dismiss);
  }, [popOpen]);

  return (
    <div
      ref={rootRef}
      onClick={onClick}
      onDoubleClick={(e) => {
        e.stopPropagation();
        // For a single-connection ghost, double-click still navigates
        // directly — same affordance as before. For multi, double-click is
        // ambiguous so we just open the popover (the single-click above
        // already opened it; the second click would close it, so we
        // re-open here for consistency).
        if (count === 1) {
          data.onNavigate?.(data.connections[0].externalComponentUid);
          setPopOpen(false);
        } else {
          setPopOpen(true);
        }
      }}
      title={
        count === 1
          ? `${first?.externalPath} · ${first?.externalPropName} — double-click to open`
          : `${count} cross-folder connections — click to expand`
      }
      style={{
        // Inner box fills whatever width the RF node was given. App.tsx sizes
        // each ghost to its content so this collapses tight around the text.
        width: "100%",
        height: GHOST_H,
        background: popOpen ? "#1a1d24" : "#0f1115",
        border: "1px dashed #5a6172",
        borderRadius: 3,
        display: "flex",
        alignItems: "center",
        padding: "0 8px",
        gap: 6,
        fontSize: 10,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        color: "#8892a0",
        whiteSpace: "nowrap",
        overflow: "hidden",
        cursor: "pointer",
        boxSizing: "border-box",
        // Make sure clicks reach us even though RF marks the node
        // non-selectable + non-draggable.
        pointerEvents: "all",
      }}
    >
      <span
        style={{
          color: "#9ecbff",
          overflow: "hidden",
          textOverflow: "ellipsis",
          minWidth: 0,
        }}
      >
        {labelLeft}
      </span>
      <span style={{ color: "#5a6172", flexShrink: 0 }}>·</span>
      <span
        style={{
          color: "#e6e8eb",
          overflow: "hidden",
          textOverflow: "ellipsis",
          minWidth: 0,
        }}
      >
        {labelRight}
      </span>
      {count > 1 && (
        <span
          style={{
            flexShrink: 0,
            fontSize: 9,
            padding: "0 4px",
            background: "#3b6eff",
            color: "#fff",
            borderRadius: 2,
            fontWeight: 600,
          }}
        >
          +{count - 1}
        </span>
      )}
      <Handle
        id={data.handleId}
        type={isInputSide ? "target" : "source"}
        position={isInputSide ? Position.Left : Position.Right}
        style={{
          width: 8,
          height: 8,
          background: "#5a6172",
          border: "1px solid #0f1115",
          borderRadius: 1,
          // Cancel React Flow's default 50% translate so the marker sits flush
          // at the ghost's edge, mirroring the real node's handle geometry.
          transform: "none",
          top: "50%",
          marginTop: -4,
          [isInputSide ? "left" : "right"]: -4,
        }}
      />
      {popOpen && rootRef.current && (
        <GhostPopover
          anchor={rootRef.current}
          isInputSide={isInputSide}
          connections={data.connections}
          onPick={(uid) => {
            setPopOpen(false);
            data.onNavigate?.(uid);
          }}
          onDeleteEdge={data.onDeleteEdge}
        />
      )}
    </div>
  );
}

// Renders below or beside the ghost when multiple cross-folder connections
// share the same handle. Each row is a navigation target on click, plus a ✕
// to delete that specific edge.
function GhostPopover({
  anchor,
  isInputSide,
  connections,
  onPick,
  onDeleteEdge,
}: {
  anchor: HTMLElement;
  isInputSide: boolean;
  connections: GhostConnection[];
  onPick: (externalUid: number) => void;
  onDeleteEdge?: (edgeUid: number) => void | Promise<void>;
}) {
  // Anchor below the ghost, aligned to its appropriate side. Using the
  // anchor's bounding rect (which is in viewport coords after RF's
  // transform) gives us correct placement at any zoom/pan.
  const rect = anchor.getBoundingClientRect();
  const top = rect.bottom + 4;
  const left = isInputSide ? rect.left : rect.right - 220;
  return createPortal(
    <div
      data-ce-ghost-pop
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        top,
        left,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        padding: 4,
        minWidth: 220,
        maxWidth: 360,
        maxHeight: 280,
        overflowY: "auto",
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 11,
        color: "#e6e8eb",
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      <div
        style={{
          padding: "4px 8px 6px 8px",
          color: "#5a6172",
          fontSize: 9,
          textTransform: "uppercase",
          letterSpacing: 0.4,
          borderBottom: "1px solid #2c313c",
          marginBottom: 4,
        }}
      >
        {connections.length} connection{connections.length === 1 ? "" : "s"}
      </div>
      {connections.map((c) => {
        const pathLabel = stripRoot(c.externalPath);
        return (
          <div
            key={c.edgeUid}
            style={{
              display: "flex",
              width: "100%",
              alignItems: "center",
              gap: 4,
              borderRadius: 2,
            }}
            onMouseEnter={(e) => (e.currentTarget.style.background = "#2c313c")}
            onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
          >
            <button
              onClick={() => onPick(c.externalComponentUid)}
              style={{
                display: "flex",
                flex: 1,
                minWidth: 0,
                alignItems: "baseline",
                gap: 6,
                padding: "4px 8px",
                background: "transparent",
                border: "none",
                color: "#e6e8eb",
                fontSize: 11,
                fontFamily: "inherit",
                cursor: "pointer",
                textAlign: "left",
              }}
              title="open this component's folder"
            >
              <span style={{ color: "#9ecbff", flexShrink: 0 }}>{pathLabel}</span>
              <span style={{ color: "#5a6172" }}>·</span>
              <span style={{ color: "#e6e8eb" }}>{c.externalPropName}</span>
            </button>
            {onDeleteEdge && (
              <button
                onClick={() => void onDeleteEdge(c.edgeUid)}
                title="delete this edge"
                style={{
                  flexShrink: 0,
                  padding: "2px 6px",
                  marginRight: 4,
                  background: "transparent",
                  border: "1px solid transparent",
                  borderRadius: 2,
                  color: "#8892a0",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  fontSize: 11,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "#3a1a1a";
                  e.currentTarget.style.color = "#ffb8b8";
                  e.currentTarget.style.borderColor = "#6b2a2a";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "#8892a0";
                  e.currentTarget.style.borderColor = "transparent";
                }}
              >
                ✕
              </button>
            )}
          </div>
        );
      })}
    </div>,
    document.body,
  );
}

export const GhostNode = memo(GhostNodeInner, (a, b) => {
  // Connections compared by reference + length; App.tsx rebuilds the array
  // on each reload so reference equality holds across unrelated updates.
  return (
    a.data.connections === b.data.connections &&
    a.data.handleId === b.data.handleId &&
    a.data.side === b.data.side &&
    a.data.width === b.data.width &&
    a.data.onNavigate === b.data.onNavigate
  );
});

// Helper for App.tsx — replicates FunctionBlock's row sort so a ghost can be
// positioned exactly at the Y of the connected property row in the visible
// component. Returns the row index, or -1 if the property isn't user-facing
// (system / non-normal systemRole — wouldn't be in any visible row).
export function userFacingRowIndex(comp: Component, propName: string): number {
  const entries = Object.entries(comp.properties).filter(
    ([, p]) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL,
  );
  const order: PropertyCategory[] = [CATEGORY_OUTPUT, CATEGORY_INPUT, CATEGORY_CONFIG];
  const sorted: string[] = [];
  for (const cat of order) {
    for (const [n, p] of entries) {
      if (p.category === cat) sorted.push(n);
    }
  }
  return sorted.indexOf(propName);
}
