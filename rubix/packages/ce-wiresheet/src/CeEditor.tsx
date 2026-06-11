import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  MiniMap,
  PanOnScrollMode,
  SelectionMode,
  applyEdgeChanges,
  applyNodeChanges,
  useReactFlow,
  useStore as useRfStore,
  type Connection,
  type Edge as RfEdge,
  type EdgeChange,
  type Node as RfNode,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

// Selected-edge highlight. RF adds .selected to .react-flow__edge when the
// edge's `selected` flag is true; the default stylesheet's selected color is
// hard to see on a dark canvas, so we paint a brighter stroke + drop shadow.
const EDGE_SELECTED_CSS = `
  .react-flow__edge.selected .react-flow__edge-path {
    stroke: #ffd166 !important;
    stroke-width: 2.5 !important;
    filter: drop-shadow(0 0 4px rgba(255,209,102,0.6));
  }
`;

import { ClickDebugger } from "./components/ClickDebugger";
import { DiagPanel } from "./components/DiagPanel";
import { EventsPanel } from "./components/EventsPanel";
import { FindPanel } from "./components/FindPanel";
import { PresenceBar } from "./components/PresenceBar";
import { ZoomRateController } from "./components/ZoomRateController";
import { VisibilitySub } from "./components/VisibilitySub";
import {
  CeWiresheetContext,
  CopyUid,
  FunctionBlock,
  GHOST_H,
  GhostNode,
  NODE_W,
  ghostWidthFor,
  stripRoot,
  userFacingRowIndex,
  type FunctionBlockData,
  type GhostNodeData,
} from "./components/FunctionBlock";
import {
  addEdge as restAddEdge,
  addNode as restAddNode,
  bulkDelete,
  bulkUpdate,
  callAction,
  copyNodes,
  restoreItems,
  getNodeByUid,
  getRootNodes,
  removeEdge as restRemoveEdge,
  removeNode as restRemoveNode,
  setEngineBase,
  setRestSessionId,
  updateEdge as restUpdateEdge,
  updateNode,
  RestError,
} from "./lib/rest";
import type { Component, Edge, FlexValue } from "./lib/engine-types";
import {
  TYPE_STATUS,
  CATEGORY_INPUT,
  CATEGORY_CONFIG,
  ROLE_NORMAL,
} from "./lib/engine-types";
import {
  loadSchemaIndices,
  useStatusFlags,
  useStructural,
  useValues,
} from "./lib/store";
import { metrics } from "./lib/instrumentation";
import {
  diagGauges,
  startDiagnostics,
  startDiagReporter,
  stopDiagnostics,
  stopDiagReporter,
} from "./lib/diagnostics";
import { usePresence, type PresenceState } from "./lib/presence";
import { CeRestWs, wsUrlFromBase } from "./lib/ws";
import {
  facetFor,
  parseFacet,
  rawFacet,
  serializeFacet,
  exposedPorts,
  FACET_PROP,
  type Alias,
  type ComponentFacet,
  type PropFacet,
} from "./lib/facet";

const nodeTypes = { fb: FunctionBlock, ghost: GhostNode };

// Constants for ghost layout. ROW_H lives in FunctionBlock; we keep the title
// height local because it's only used here for ghost-Y math (it's also defined
// there for the node itself).
const FB_TITLE_H = 40;
const FB_ROW_H = 18;
// Horizontal gap between a visible component and its ghost sub-node.
const GHOST_GAP = 60;

// Module-level WS singleton so HMR / StrictMode double mounts don't open two sockets.
let wsClient: CeRestWs | null = null;

// MIME type for drag-and-drop from the palette into the React Flow canvas. Custom so
// we don't conflict with any other drop sources (text, files, etc).
const DND_TYPE = "application/x-ce-component-type";

interface PaletteComponent {
  name: string;
  type: string; // full "vendor-ext::name"
  icon?: string;
}

interface PaletteExtension {
  id: string; // "vendor-ext"
  vendor: string;
  name: string;
  version?: string;
  components: PaletteComponent[];
}

// Action spec for a component type, sourced from `/schema`. The signature
// (params/returns) is static per type, so it rides the cached schema — opening
// the picker is a pure in-memory lookup, no per-right-click fetch. Future
// per-instance availability layers on top via `getActionsFor` without changing
// this shape (see the action-discovery design notes).
interface ActionParamDef {
  name: string;
  type: string; // FlexValue type: bool | int | i32 | i64 | f32 | f64 | str | ...
  default?: FlexValue;
  label?: string;
  required?: boolean;
  enum?: FlexValue[];
}
interface ActionReturnDef {
  name: string;
  type: string;
}
interface ActionDef {
  name: string;
  label?: string;
  description?: string;
  params?: ActionParamDef[];
  returns?: ActionReturnDef[];
}

// Root component UID. The engine's root is always 0.
const ROOT_UID = 0;

// Pointer travel (px) that turns a right-press into a marquee drag rather than a
// click. Generous so a normal click's jitter never registers as a drag — a real
// drag-select moves much further. The marquee select and the contextmenu
// suppression key off the SAME activation, so they never both fire.
const MARQUEE_DRAG_PX = 8;

// Per-tab discriminator for the presence display name. Generated once at module
// load — unique per tab (a duplicated tab re-runs module init, so it differs
// even though it copied storage). Short, derived from the high-res clock.
const TAB_SUFFIX = Math.trunc(performance.now() * 1000 + performance.timeOrigin)
  .toString(36)
  .slice(-4);

// `base` is the selected control engine's REST origin, e.g.
// `http://192.168.1.50:7878`. The standalone harness passes its own (proxied)
// origin; the extension passes the selected device's `ip:port`.
export default function CeEditor({ base }: { base: string }) {
  return (
    <ReactFlowProvider>
      <Inner base={base} />
    </ReactFlowProvider>
  );
}

interface Crumb {
  uid: number;
  name: string;
}

function Inner({ base }: { base: string }) {
  // Point the REST client at the selected engine before any request fires.
  // useMemo runs during render (ahead of the effects that call reload()), so
  // the first fetch already targets `<base>/api/v0`.
  useMemo(() => setEngineBase(base), [base]);

  // Nodes contain both real function blocks AND ghost sub-nodes for cross-folder
  // edge endpoints. RF's nodeTypes routes each to its renderer by `type`.
  type AnyNode = RfNode<FunctionBlockData> | RfNode<GhostNodeData>;
  const [nodes, setNodes] = useState<AnyNode[]>([]);
  const [edges, setEdges] = useState<RfEdge[]>([]);
  // Edges from the last reload, parked here until every node has been measured.
  // Without this gate React Flow tries to place edges before handles exist in its
  // internal lookup → "Couldn't create edge for source handle id …" warning.
  const [pendingEdges, setPendingEdges] = useState<RfEdge[] | null>(null);
  // exposed child-prop uid → its owning child COMPONENT uid, for the current view.
  // Lets onConnect target the real child (not the folder the port is drawn on)
  // when wiring to an exposed port. Populated in reload.
  const exposedRemapRef = useRef<Map<number, number>>(new Map());

  // Our WS session id; used to distinguish own echo (instant snap) from remote
  // topology changes (animate). Set from the schema callback below.
  const sessionIdRef = useRef<string | null>(null);

  // Position-tween state for remote-origin position changes. Keyed by node id.
  //
  // We use a critically-damped exponential ease instead of a fixed-duration curve:
  // on each rAF tick the node's current position moves toward the target by a
  // fraction `1 - exp(-RATE * dt)`. Properties of this approach that fixed-duration
  // easeOut doesn't have:
  //   - Velocity-aware retargeting. A second position update arriving mid-flight
  //     doesn't restart from rest — the easing simply pulls toward the new target
  //     from wherever the node currently is. No double-tween hitch.
  //   - Frame-independent. Reads dt from the rAF clock, so a dropped frame just
  //     means the next tick takes a bigger step; no time wobble.
  //   - Settles asymptotically. We snap to target and drop the entry when both
  //     axes are within 0.5px.
  // RATE is per-second; higher = snappier settle. 9 ≈ ~400ms to within 1% of target.
  const POS_ANIM_RATE = 9;
  const POS_SETTLE_PX = 0.5;
  const posAnims = useRef(
    new Map<
      string,
      { curPos: { x: number; y: number }; endPos: { x: number; y: number } }
    >(),
  );
  const posAnimRaf = useRef<number | null>(null);
  const posAnimLastTick = useRef<number | null>(null);

  const tickPosAnims = useCallback(() => {
    const now = performance.now();
    const last = posAnimLastTick.current;
    // Clamp dt so an idle tab returning to focus doesn't make the next step
    // jump the whole remaining distance in one frame.
    const dt = last != null ? Math.min(0.05, (now - last) / 1000) : 1 / 60;
    posAnimLastTick.current = now;
    const anims = posAnims.current;
    if (anims.size === 0) {
      posAnimRaf.current = null;
      posAnimLastTick.current = null;
      return;
    }
    const alpha = 1 - Math.exp(-POS_ANIM_RATE * dt);
    const patch = new Map<string, { x: number; y: number }>();
    for (const [id, a] of anims) {
      const nx = a.curPos.x + (a.endPos.x - a.curPos.x) * alpha;
      const ny = a.curPos.y + (a.endPos.y - a.curPos.y) * alpha;
      if (
        Math.abs(a.endPos.x - nx) < POS_SETTLE_PX &&
        Math.abs(a.endPos.y - ny) < POS_SETTLE_PX
      ) {
        patch.set(id, a.endPos);
        anims.delete(id);
      } else {
        a.curPos = { x: nx, y: ny };
        patch.set(id, { x: nx, y: ny });
      }
    }
    if (patch.size > 0) {
      setNodes((ns) =>
        ns.map((n) => {
          // Anchor components — apply the tween position from the patch.
          const p = patch.get(n.id);
          if (p) return { ...n, position: p };
          // Ghosts anchored to a moving component follow along so the edge
          // stub stays glued during cross-window position animations.
          if (n.type === "ghost") {
            const g = n as RfNode<GhostNodeData>;
            const anchor = patch.get(String(g.data.anchorUid));
            if (!anchor) return n;
            const gx =
              g.data.side === "input"
                ? anchor.x + NODE_W + GHOST_GAP
                : anchor.x - g.data.width - GHOST_GAP;
            const gy = anchor.y + FB_TITLE_H + g.data.anchorRowIdx * FB_ROW_H;
            return { ...g, position: { x: gx, y: gy } };
          }
          return n;
        }),
      );
    }
    posAnimRaf.current = anims.size > 0 ? requestAnimationFrame(tickPosAnims) : null;
    if (anims.size === 0) posAnimLastTick.current = null;
  }, []);

  const animateNodeTo = useCallback(
    (id: string, fromPos: { x: number; y: number }, toPos: { x: number; y: number }) => {
      const existing = posAnims.current.get(id);
      // Retarget: keep `curPos` if we're already animating (preserves visual
      // continuity — the new pull starts from wherever the node currently is,
      // not from the call site's `fromPos` which lags React render).
      posAnims.current.set(id, {
        curPos: existing ? existing.curPos : fromPos,
        endPos: toPos,
      });
      if (posAnimRaf.current == null) {
        posAnimRaf.current = requestAnimationFrame(tickPosAnims);
      }
    },
    [tickPosAnims],
  );

  useEffect(() => {
    return () => {
      if (posAnimRaf.current != null) {
        cancelAnimationFrame(posAnimRaf.current);
        posAnimRaf.current = null;
      }
      posAnims.current.clear();
      posAnimLastTick.current = null;
    };
  }, []);

  const [error, setError] = useState<{ message: string; debug?: string } | null>(null);
  // Normalise any caught value into the banner shape; RestErrors carry a
  // copy-pasteable request/response dump for debugging.
  const reportError = useCallback((e: unknown) => {
    if (e instanceof RestError) setError({ message: e.message, debug: e.debug });
    else setError({ message: e instanceof Error ? e.message : String(e) });
  }, []);
  const [palette, setPalette] = useState<PaletteExtension[]>([]);
  // Action signatures indexed by component type, built from the same `/schema`
  // pass as the palette. Read on right-click — no per-open fetch.
  const [actionsByType, setActionsByType] = useState<Map<string, ActionDef[]>>(
    () => new Map(),
  );
  // Same info as a type-set in a ref, so `buildRfNodes` can stamp `hasActions`
  // without making `reload` depend on the (async-loaded) action index.
  const actionTypesRef = useRef<Set<string>>(new Set());
  const [crumbs, setCrumbs] = useState<Crumb[]>([{ uid: ROOT_UID, name: "root" }]);
  const currentParentUid = crumbs[crumbs.length - 1].uid;
  const rf = useReactFlow();

  // Mouse model: LEFT-drag on the pane pans (panOnDrag={[0]}); RIGHT-drag on the
  // pane is a marquee box-select. (React Flow's built-in selectionOnDrag is
  // left-button-only, so the right-button marquee is implemented here.)
  //
  // Direction convention (CAD-style): left-to-right marquee = "fully enclosed"
  // only; right-to-left = "touching" too. Maps to getIntersectingNodes's
  // `partially` flag.
  const marquee = useRef<{ startX: number; startY: number; active: boolean } | null>(null);
  const [marqueeRect, setMarqueeRect] = useState<
    { x: number; y: number; w: number; h: number } | null
  >(null);

  // Is this pointer event on the empty pane (not a node / edge / handle)? Only
  // then does a right-drag start a marquee — right-clicking a node/edge/row
  // still opens its context menu.
  const isPaneTarget = (target: EventTarget | null): boolean => {
    let el = target as Element | null;
    while (el) {
      if (el.classList?.contains("react-flow__node")) return false;
      if (el.classList?.contains("react-flow__edge")) return false;
      if (el.classList?.contains("react-flow__handle")) return false;
      if (el.classList?.contains("react-flow__pane")) return true;
      el = el.parentElement;
    }
    return false;
  };

  const onCanvasPointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button === 2 && isPaneTarget(e.target)) {
      marquee.current = { startX: e.clientX, startY: e.clientY, active: false };
    }
  }, []);
  const onCanvasPointerMove = useCallback((e: React.PointerEvent) => {
    const m = marquee.current;
    if (!m) return;
    const dx = e.clientX - m.startX;
    const dy = e.clientY - m.startY;
    if (!m.active && Math.hypot(dx, dy) < MARQUEE_DRAG_PX) return; // not a drag yet
    m.active = true;
    setMarqueeRect({
      x: Math.min(m.startX, e.clientX),
      y: Math.min(m.startY, e.clientY),
      w: Math.abs(dx),
      h: Math.abs(dy),
    });
  }, []);
  const onCanvasPointerUp = useCallback(
    (e: React.PointerEvent) => {
      const m = marquee.current;
      marquee.current = null;
      if (!m) return; // not a right-pane gesture
      if (!m.active) {
        // Quick right-click (no drag) → open the pane menu HERE, on release.
        // The browser fires `contextmenu` on PRESS — before we know whether the
        // gesture becomes a drag-select — so the menu can't be opened there
        // without also firing on drags. Deciding at pointer-up fixes that.
        setMarqueeRect(null);
        setNodeMenu(null);
        setPaneMenu({ x: e.clientX, y: e.clientY });
        return;
      }
      // Drag → marquee selection. Screen rect → flow rect via the two corners.
      const a = rf.screenToFlowPosition({ x: m.startX, y: m.startY });
      const b = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      const rect = {
        x: Math.min(a.x, b.x),
        y: Math.min(a.y, b.y),
        width: Math.abs(b.x - a.x),
        height: Math.abs(b.y - a.y),
      };
      // left-to-right drag → fully-enclosed only; right-to-left → touching.
      const partially = e.clientX < m.startX;
      const hits = rf.getIntersectingNodes(rect, partially);
      const hitIds = new Set(hits.filter((n) => n.type !== "ghost").map((n) => n.id));
      const multi = e.shiftKey || e.metaKey || e.ctrlKey;
      setNodes((ns) =>
        ns.map((n) => {
          if (n.type === "ghost") return n;
          const want = multi ? n.selected || hitIds.has(n.id) : hitIds.has(n.id);
          return n.selected === want ? n : { ...n, selected: want };
        }),
      );
      if (!multi) setEdges((es) => es.map((ed) => (ed.selected ? { ...ed, selected: false } : ed)));
      setMarqueeRect(null);
    },
    [rf],
  );

  // Document-level click selection. Runs in CAPTURE phase, before React Flow's own
  // handlers, so `selectionOnDrag` can't swallow the event. We track pointerdown +
  // pointerup positions to decide "click" (under 4px movement) and apply selection
  // ourselves. RF's own selection emits land on top harmlessly via setNodes equality.
  //
  // Two clickable targets are tracked: nodes (.react-flow__node) and edges
  // (.react-flow__edge). Edges resolve through their interaction overlay path —
  // RF renders a thick invisible <path class="react-flow__edge-interaction"> as a
  // hit zone on top of the visible stroke.
  useEffect(() => {
    type HitTarget =
      | { kind: "node"; id: string }
      | { kind: "edge"; id: string }
      | null;
    const findHit = (target: EventTarget | null): HitTarget => {
      let el = target as Element | null;
      while (el) {
        if (el.classList?.contains("react-flow__node")) {
          const id = (el as HTMLElement).dataset.id ?? null;
          return id ? { kind: "node", id } : null;
        }
        if (el.classList?.contains("react-flow__edge")) {
          const id = (el as HTMLElement).dataset.id ?? null;
          return id ? { kind: "edge", id } : null;
        }
        if (el.classList?.contains("react-flow__pane")) return null;
        el = el.parentElement;
      }
      return null;
    };
    const isPane = (target: EventTarget | null): boolean => {
      let el = target as Element | null;
      while (el) {
        if (el.classList?.contains("react-flow__pane")) return true;
        if (el.classList?.contains("react-flow__node")) return false;
        if (el.classList?.contains("react-flow__edge")) return false;
        el = el.parentElement;
      }
      return false;
    };
    let downAt: { x: number; y: number; hit: HitTarget } | null = null;
    const onDown = (e: PointerEvent) => {
      if (e.button !== 0) {
        downAt = null;
        return;
      }
      downAt = { x: e.clientX, y: e.clientY, hit: findHit(e.target) };
    };
    const onUp = (e: PointerEvent) => {
      const d = downAt;
      downAt = null;
      if (!d) return;
      const dist = Math.hypot(e.clientX - d.x, e.clientY - d.y);
      if (dist > 4) return; // a drag, not a click

      const upHit = findHit(e.target);
      const multi = e.shiftKey || e.metaKey || e.ctrlKey;

      // Click resolved on the same node — toggle selection.
      if (d.hit?.kind === "node" && upHit?.kind === "node" && upHit.id === d.hit.id) {
        const id = d.hit.id;
        // Ghosts handle their own clicks (popover). Leave them alone so the
        // doc-level handler doesn't flip their selection state under the
        // popover's feet, and so React's onClick on the ghost actually fires.
        if (id.startsWith("ghost:")) return;
        metrics.lastSelChange = `click→${useStructural.getState().components.get(Number(id))?.name ?? id} (capture)`;
        metrics.lastSelChangeAt = performance.now();
        setNodes((ns) =>
          ns.map((n) => {
            if (multi) return n.id === id ? { ...n, selected: !n.selected } : n;
            const want = n.id === id;
            return n.selected === want ? n : { ...n, selected: want };
          }),
        );
        if (!multi) {
          setEdges((es) => es.map((edge) => (edge.selected ? { ...edge, selected: false } : edge)));
        }
        return;
      }

      // Click resolved on the same edge — toggle selection.
      if (d.hit?.kind === "edge" && upHit?.kind === "edge" && upHit.id === d.hit.id) {
        const id = d.hit.id;
        metrics.lastSelChange = `edge→${id}`;
        metrics.lastSelChangeAt = performance.now();
        setEdges((es) =>
          es.map((edge) => {
            if (multi) return edge.id === id ? { ...edge, selected: !edge.selected } : edge;
            const want = edge.id === id;
            return edge.selected === want ? edge : { ...edge, selected: want };
          }),
        );
        if (!multi) {
          setNodes((ns) => ns.map((n) => (n.selected ? { ...n, selected: false } : n)));
        }
        return;
      }

      // Clean click on the pane → clear selection.
      if (!d.hit && !upHit && isPane(e.target)) {
        metrics.lastSelChange = "pane→clear (capture)";
        metrics.lastSelChangeAt = performance.now();
        setNodes((ns) => ns.map((n) => (n.selected ? { ...n, selected: false } : n)));
        setEdges((es) => es.map((edge) => (edge.selected ? { ...edge, selected: false } : edge)));
      }
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("pointerup", onUp, true);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("pointerup", onUp, true);
    };
  }, []);

  // Escape clears the current selection on both nodes and edges. React Flow doesn't
  // bind this by default. Listening at the window level so it works regardless of
  // focus state (clicking on a node usually doesn't move focus to the canvas).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // If the focus is in an input/textarea (e.g. the palette filter), let Esc do
      // whatever the input wants instead of clearing canvas selection.
      const ae = document.activeElement;
      if (ae && (ae.tagName === "INPUT" || ae.tagName === "TEXTAREA" || (ae as HTMLElement).isContentEditable)) {
        return;
      }
      setNodes((ns) => ns.map((n) => (n.selected ? { ...n, selected: false } : n)));
      setEdges((es) => es.map((edge) => (edge.selected ? { ...edge, selected: false } : edge)));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Click-into handler — pushed into each block's `data.onEnter` so a memo equality
  // check on identity is stable across renders for a given crumb stack.
  const enter = useCallback((uid: number) => {
    const c = useStructural.getState().components.get(uid);
    if (!c) return;
    setCrumbs((cur) => [...cur, { uid: c.uid, name: c.name || c.type }]);
  }, []);

  // After a navigation jump (ghost double-click), the target component lives
  // in a different folder than the current view. We set this so the
  // post-reload effect below knows which node to center + select once the
  // new view's nodes are mounted.
  const [focusAfterLoad, setFocusAfterLoad] = useState<number | null>(null);

  // Component finder (Cmd/Ctrl+F). Searches the whole tree, jumps to a pick.
  const [findOpen, setFindOpen] = useState(false);

  // Click-debug overlay (the selection-diagnostics rings + bottom-right log).
  // Off by default — it was for chasing the marquee/select bugs (now fixed) and
  // it sits over the minimap. Toggle with Cmd/Ctrl+Shift+D. Persisted.
  const [clickDebugOpen, setClickDebugOpen] = useState(() => {
    try {
      return window.localStorage.getItem("ce-ui.clickDebug") === "1";
    } catch {
      return false;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.clickDebug", clickDebugOpen ? "1" : "0");
    } catch {
      /* ignore */
    }
  }, [clickDebugOpen]);

  // Newly-pasted component uids waiting to land in the next reload's nodes
  // array. The post-reload effect promotes them to the active selection
  // once they appear, then clears this so unrelated nodes updates don't
  // re-trigger.
  const [pendingPasteSelection, setPendingPasteSelection] = useState<number[] | null>(null);

  // Navigate to a component's containing folder. Used by ghost sub-nodes
  // (cross-folder edge endpoints) on double-click. The component may live in
  // a folder several levels up from the current view, so we walk the
  // ancestor chain via REST to build a full crumb stack (otherwise the
  // breadcrumb would lie about depth). One REST call per ancestor — fine for
  // a user-initiated jump.
  const goToComponent = useCallback(async (uid: number) => {
    try {
      const targetResp = await getNodeByUid(uid, { depth: 0 });
      const target = targetResp.nodes[0];
      if (!target) return;
      // Walk up from target.parent to root, recording {uid, name} at each
      // level. Stop at root (uid 0).
      const chain: Crumb[] = [];
      let cursor = target.parent;
      while (cursor !== ROOT_UID) {
        const r = await getNodeByUid(cursor, { depth: 0 });
        const c = r.nodes[0];
        if (!c) break;
        chain.unshift({ uid: c.uid, name: c.name || c.type });
        if (c.parent === c.uid) break; // defensive
        cursor = c.parent;
      }
      // Mark the target so the post-reload effect picks it up. Set BEFORE
      // setCrumbs so the effect, which depends on `nodes`, finds the focus
      // request already armed when the new node list lands.
      setFocusAfterLoad(uid);
      setCrumbs([{ uid: ROOT_UID, name: "root" }, ...chain]);
    } catch (e) {
      reportError(e);
    }
  }, []);

  // Node-level right-click context menu state. Opened by a FunctionBlock when
  // the user right-clicks the body (not a property row). Lives at App level so
  // the menu / picker can read the current multi-selection from `nodes`.
  const [nodeMenu, setNodeMenu] = useState<{ x: number; y: number; uid: number } | null>(
    null,
  );
  const [movePickerOpen, setMovePickerOpen] = useState(false);
  const [actionPickerOpen, setActionPickerOpen] = useState(false);
  const [detailsUid, setDetailsUid] = useState<number | null>(null);
  // Right-click on empty pane → menu (up a folder / add component / paste).
  const [paneMenu, setPaneMenu] = useState<{ x: number; y: number } | null>(null);
  const openNodeContextMenu = useCallback(
    (uid: number, x: number, y: number) => {
      setNodeMenu({ x, y, uid });
      setPaneMenu(null);
      // Fresh menu — never reopen straight into a sub-picker.
      setMovePickerOpen(false);
      setActionPickerOpen(false);
      // Right-click acts on the right-clicked node by default; if it's already
      // in a multi-selection we leave that selection intact. If it's NOT
      // selected, replace the selection with just this node so the action's
      // target is unambiguous.
      setNodes((ns) => {
        const target = ns.find((n) => n.id === String(uid));
        if (target?.selected) return ns;
        return ns.map((n) => {
          const want = n.id === String(uid);
          return n.selected === want ? n : { ...n, selected: want };
        });
      });
    },
    [],
  );

  // The action-discovery seam. TODAY: actions are type-level, so resolve each
  // selected component's type → its signatures from the cached schema, and
  // return the actions common to ALL targets (so the chosen action exists on
  // every one). LATER (per-instance/dynamic actions): filter/merge this by a
  // live `Map<uid, availableActionNames>` (fetched on open or WS-pushed) — only
  // this function changes; the picker and invoke path stay the same.
  const getActionsFor = useCallback(
    (uids: number[]): ActionDef[] => {
      const comps = useStructural.getState().components;
      const lists = uids
        .map((u) => comps.get(u)?.type)
        .filter((t): t is string => !!t)
        .map((t) => actionsByType.get(t) ?? []);
      if (lists.length === 0) return [];
      const [first, ...rest] = lists;
      return first.filter((a) => rest.every((l) => l.some((b) => b.name === a.name)));
    },
    [actionsByType],
  );

  // Dispatch one action on every target. Actions are per-component, so we fan
  // out one `POST /call` per uid (the bulk endpoints don't dispatch actions).
  const invokeAction = useCallback(
    (uids: number[], action: string, params: Record<string, FlexValue>) =>
      Promise.all(uids.map((u) => callAction(u, action, params))),
    [],
  );

  const goToCrumb = useCallback((idx: number) => {
    setCrumbs((cur) => cur.slice(0, idx + 1));
  }, []);

  // Copy = remember the selected component UIDs + their on-screen centroid.
  // Paste clones them server-side via POST /copy/nodes (full fidelity, internal
  // edges auto-included), so the clipboard is just the uid list — no
  // value/edge snapshotting. The centroid lets paste translate the clones so
  // their group lands at the cursor preserving relative layout.
  const copySelectionToClipboard = useCallback(() => {
    const selectedReal = nodes.filter((n) => n.selected && n.type !== "ghost");
    if (selectedReal.length === 0) return;
    const uids = selectedReal.map((n) => Number(n.id));
    const xs = selectedReal.map((n) => n.position.x);
    const ys = selectedReal.map((n) => n.position.y);
    const centroid = {
      x: (Math.min(...xs) + Math.max(...xs)) / 2,
      y: (Math.min(...ys) + Math.max(...ys)) / 2,
    };
    clipboardRef.current = { uids, centroid };
    metrics.lastSelChange = `copied ${uids.length}c`;
    metrics.lastSelChangeAt = performance.now();
  }, [nodes]);

  // (pasteFromClipboard and the Cmd+C/V keyboard listener are declared
  // below — they depend on `reload`, which is itself declared further down.)

  // Delete a single cross-folder edge backing one of a ghost's connections.
  // Same REST + local-state pattern as onEdgesDelete, plus shrinks the
  // ghost's connections list (or removes the ghost entirely when its last
  // connection goes — "ghost has nothing left to point at"). Declared above
  // reload() so reload() can wire it into ghost data on every rebuild.
  const deleteGhostEdge = useCallback(async (edgeUid: number) => {
    try {
      await restRemoveEdge(edgeUid);
    } catch (e) {
      reportError(e);
      return;
    }
    useStructural.getState().removeEdge(edgeUid);
    setEdges((es) => es.filter((e) => e.id !== String(edgeUid)));
    setNodes((ns) =>
      ns.flatMap((n) => {
        if (n.type !== "ghost") return [n];
        const g = n as RfNode<GhostNodeData>;
        const idx = g.data.connections.findIndex((c) => c.edgeUid === edgeUid);
        if (idx < 0) return [n];
        const next = g.data.connections.filter((_, i) => i !== idx);
        if (next.length === 0) return []; // last edge gone — drop the ghost
        return [{ ...g, data: { ...g.data, connections: next } }];
      }),
    );
  }, []);

  // Load the children of the current parent. depth=1, nested=true gets the parent +
  // its immediate children with `childrenCount` populated. We render only the children
  // (not the parent itself), so the user is "inside" the parent's container.
  const reload = useCallback(async () => {
    try {
      let resp;
      if (currentParentUid === ROOT_UID) {
        resp = await getRootNodes({ depth: 1, nested: true, withEdges: true });
      } else {
        // withEdges: true makes /nodes/uid/X return every edge with at least
        // one endpoint inside this subtree — INCLUDING cross-folder edges
        // that reach outside. GET /edges?component=X scopes too tightly (it
        // only returns edges entirely within the subtree), so cross-folder
        // ghosts wouldn't render when viewing a child folder.
        resp = await getNodeByUid(currentParentUid, {
          depth: 1,
          nested: true,
          withEdges: true,
        });
      }
      const parent = resp.nodes[0];
      const children = parent?.children ?? [];
      const scopedEdges: Edge[] = resp.edges ?? [];
      const childUids = new Set(children.map((c) => c.uid));
      const childByUid = new Map(children.map((c) => [c.uid, c]));
      // Partition edges:
      //   - inEdges:    both endpoints visible → drawn normally.
      //   - crossEdges: one endpoint visible, the other off-canvas → drawn
      //                 against a ghost sub-node placed at the row Y of the
      //                 visible endpoint.
      const inEdges: Edge[] = [];
      const crossEdges: Edge[] = [];
      for (const e of scopedEdges) {
        const src = childUids.has(e.sourceUid);
        const dst = childUids.has(e.targetUid);
        if (src && dst) inEdges.push(e);
        else if (src !== dst) crossEdges.push(e);
        // Both off-canvas: skip — neither endpoint is in this view at all.
      }
      useStructural.getState().setNodes(children, inEdges);

      // Build ghost nodes + their cross-folder edges. Cross-folder edges that
      // share the same visible-side (component, property) are GROUPED into one
      // ghost so an output that fans out to N external inputs doesn't render
      // N overlapping ghost boxes at the same Y. The ghost shows the first
      // target inline and surfaces the rest in a click-to-expand popover.
      interface GhostGroup {
        visibleUid: number;
        visiblePropUid: number;
        rowIdx: number;
        side: "input" | "output";
        connections: import("./components/FunctionBlock").GhostConnection[];
        edgeUids: number[];
        visibleX: number;
        visibleY: number;
      }
      // Reverse index of exposed ports in THIS view: a child-prop uid → the
      // visible component (e.g. a folder) that projects it as a port. A
      // cross-folder edge whose off-canvas end is one of these attaches to that
      // component's port handle (handle id = the child prop uid) and renders as a
      // normal edge, instead of a ghost. (FACET_DESIGN.md §9.)
      const exposedIndex = new Map<number, { parentUid: number }>();
      const exposedRemap = new Map<number, number>();
      const subProps = new Set<number>();
      for (const child of children) {
        for (const ep of exposedPorts(facetFor(child.uid, rawFacet(child.properties)))) {
          exposedIndex.set(ep.childUid, { parentUid: child.uid });
          if (ep.facet.childComponent != null) exposedRemap.set(ep.childUid, ep.facet.childComponent);
          subProps.add(ep.childUid); // the port's live value
          if (ep.facet.facetProp != null) subProps.add(ep.facet.facetProp); // child's live __facets
        }
      }
      exposedRemapRef.current = exposedRemap;
      // Subscribe the exposed (off-canvas) child props AND their child's __facets
      // prop at PROPERTY level so both the value and the presentation metadata
      // stream live. (Component-level subs only cover visible nodes.)
      wsClient?.setDesiredPropSubscription(subProps);
      const portEdges: RfEdge[] = [];

      const ghostGroups = new Map<string, GhostGroup>();
      for (const e of crossEdges) {
        const externalIsTarget = childUids.has(e.sourceUid);
        // If the off-canvas end is an exposed port, draw a normal edge to the
        // exposing component's port handle and skip the ghost.
        const externalPropUid = externalIsTarget ? e.targetPropertyUid : e.sourcePropertyUid;
        const exposed = externalPropUid != null ? exposedIndex.get(externalPropUid) : undefined;
        if (exposed) {
          const vUid = externalIsTarget ? e.sourceUid : e.targetUid;
          const vPropUid = externalIsTarget ? e.sourcePropertyUid : e.targetPropertyUid;
          if (vPropUid != null) {
            const style =
              e.loopBack === true
                ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" }
                : { stroke: "#4a9eff", strokeWidth: 1.5 };
            portEdges.push(
              externalIsTarget
                ? {
                    id: String(e.uid),
                    source: String(vUid),
                    sourceHandle: String(vPropUid),
                    target: String(exposed.parentUid),
                    targetHandle: String(externalPropUid),
                    style,
                    animated: false,
                  }
                : {
                    id: String(e.uid),
                    source: String(exposed.parentUid),
                    sourceHandle: String(externalPropUid),
                    target: String(vUid),
                    targetHandle: String(vPropUid),
                    style,
                    animated: false,
                  },
            );
            continue;
          }
        }
        const visibleUid = externalIsTarget ? e.sourceUid : e.targetUid;
        const externalUid = externalIsTarget ? e.targetUid : e.sourceUid;
        const visibleComp = childByUid.get(visibleUid);
        if (!visibleComp) continue;
        const propName = externalIsTarget ? e.sourceProperty : e.targetProperty;
        const visibleProp = visibleComp.properties[propName];
        if (!visibleProp) continue;
        const rowIdx = userFacingRowIndex(visibleComp, propName);
        if (rowIdx < 0) continue;
        const externalPropName = externalIsTarget ? e.targetProperty : e.sourceProperty;
        const externalPath = externalIsTarget ? e.targetPath ?? "" : e.sourcePath ?? "";
        // Group key: visible side identifies the ghost. Multiple edges that
        // share this side merge into one ghost.
        const key = `${visibleUid}:${visibleProp.uid}`;
        let group = ghostGroups.get(key);
        if (!group) {
          group = {
            visibleUid,
            visiblePropUid: visibleProp.uid,
            rowIdx,
            side: externalIsTarget ? "input" : "output",
            connections: [],
            edgeUids: [],
            visibleX: visibleComp.metadata?.position?.x ?? 0,
            visibleY: visibleComp.metadata?.position?.y ?? 0,
          };
          ghostGroups.set(key, group);
        }
        group.connections.push({
          externalComponentUid: externalUid,
          externalPath,
          externalPropName,
          edgeUid: e.uid,
        });
        group.edgeUids.push(e.uid);
      }

      const ghostNodes: RfNode<GhostNodeData>[] = [];
      const ghostEdges: RfEdge[] = [];
      for (const g of ghostGroups.values()) {
        // Width tailored to the FIRST connection's label (collapsed-state
        // display): the full path (with leading "root/" stripped) + prop
        // name. Popover shows the rest at full width.
        const first = g.connections[0];
        const labelPath = stripRoot(first.externalPath);
        const gw = ghostWidthFor(labelPath, first.externalPropName) + (g.connections.length > 1 ? 26 : 0);
        const gx = g.side === "input" ? g.visibleX + NODE_W + GHOST_GAP : g.visibleX - gw - GHOST_GAP;
        const gy = g.visibleY + FB_TITLE_H + g.rowIdx * FB_ROW_H + (FB_ROW_H - GHOST_H) / 2;
        const ghostId = `ghost:${g.visibleUid}:${g.visiblePropUid}`;
        const handleId = `gh:${g.visibleUid}:${g.visiblePropUid}`;
        ghostNodes.push({
          id: ghostId,
          type: "ghost",
          position: { x: gx, y: gy },
          width: gw,
          // selectable: false would strip pointer events on the wrapper in
          // some RF configs, defeating the popover. Keep selectable + harmless;
          // the doc-level click handler skips ghost ids so it doesn't latch
          // selection visually. Still non-draggable.
          draggable: false,
          data: {
            connections: g.connections,
            handleId,
            side: g.side,
            anchorUid: g.visibleUid,
            anchorRowIdx: g.rowIdx,
            width: gw,
            onNavigate: goToComponent,
            onDeleteEdge: deleteGhostEdge,
          },
        });
        // All edges in this group share the same ghost handle id. The visible
        // end uses the real prop uid as its handle id (FunctionBlock renders
        // its Handles with id={String(p.uid)}).
        const visibleHandleId = String(g.visiblePropUid);
        for (const edgeUid of g.edgeUids) {
          // Reconstruct edge from the same crossEdges entry so we have the
          // loopBack / stroke choice. Slightly redundant but keeps the
          // grouping logic linear.
          const e = crossEdges.find((x) => x.uid === edgeUid)!;
          const externalIsTarget = g.side === "input";
          ghostEdges.push({
            id: String(edgeUid),
            source: externalIsTarget ? String(g.visibleUid) : ghostId,
            sourceHandle: externalIsTarget ? visibleHandleId : handleId,
            target: externalIsTarget ? ghostId : String(g.visibleUid),
            targetHandle: externalIsTarget ? handleId : visibleHandleId,
            style:
              e.loopBack === true
                ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" }
                : { stroke: "#4a9eff", strokeWidth: 1.5 },
            animated: false,
          });
        }
      }
      // Capture current selection by passing the existing nodes array to the builder.
      // Using the functional form of setNodes guarantees we see the latest state at
      // the moment React applies it — critical when a click and a topology reload
      // batch together.
      setNodes((prev) => {
        const selectedIds = new Set<string>();
        for (const n of prev) if (n.selected) selectedIds.add(n.id);
        const real = buildRfNodes(
          children,
          enter,
          openNodeContextMenu,
          selectedIds,
          actionTypesRef.current,
        );
        return [...real, ...ghostNodes];
      });
      // Stash edges; the useNodesInitialized effect below will move them into the live
      // `edges` state once React Flow has registered handle positions for every node.
      setEdges([]);
      setPendingEdges([...buildRfEdges(inEdges, children), ...ghostEdges, ...portEdges]);
      // NOTE: subscription is no longer set here. VisibilitySub owns it — it
      // subscribes only the components in/near the viewport (debounced), which
      // fires shortly after these nodes mount. Subscribing all folder children
      // here would stream the off-screen majority for nothing.
    } catch (e) {
      reportError(e);
    }
  }, [currentParentUid, enter, openNodeContextMenu, goToComponent, deleteGhostEdge]);

  // The WS effect captures its handlers once (`[]` deps), so anything it calls
  // must reach the LATEST reload — otherwise a topology event (e.g. after adding
  // a component) fires a stale reload bound to the root level and snaps the view
  // back to root. Keep a ref to the current reload for those call sites.
  const reloadRef = useRef(reload);
  reloadRef.current = reload;

  // Paste = server-side clone of the copied components via POST /copy/nodes
  // (internal edges auto-included), then a single bulkUpdate to translate the
  // clones so their group centroid lands at the cursor (preserving relative
  // layout). The engine auto-suffixes names and assigns uids — no client-side
  // spec building, name collision handling, or edge reconstruction.
  const pasteFromClipboard = useCallback(async () => {
    const cb = clipboardRef.current;
    if (!cb || cb.uids.length === 0) return;
    try {
      const res = await copyNodes({
        componentUids: cb.uids,
        destParentUid: currentParentUid,
        includeInternalEdges: true,
      });
      const clones = res.nodes ?? [];
      if (clones.length === 0) {
        setError({ message: "paste: nothing cloned (sources may have been deleted)" });
        return;
      }
      // Translate the clones so their centroid lands at the cursor. The clones
      // come back at the SOURCE positions; offset = cursor − source centroid
      // (we kept the source centroid at copy time, but recompute from the
      // clones to be robust to any engine repositioning).
      const cursor = rf.screenToFlowPosition(mouseScreenPos.current);
      const xs = clones.map((c) => c.metadata?.position?.x ?? 0);
      const ys = clones.map((c) => c.metadata?.position?.y ?? 0);
      const dx = cursor.x - (Math.min(...xs) + Math.max(...xs)) / 2;
      const dy = cursor.y - (Math.min(...ys) + Math.max(...ys)) / 2;
      const updates = clones.map((c) => ({
        uid: c.uid,
        position: {
          x: Math.round((c.metadata?.position?.x ?? 0) + dx),
          y: Math.round((c.metadata?.position?.y ?? 0) + dy),
        },
      }));
      await bulkUpdate(updates);
      const newUids = clones.map((c) => c.uid);
      setPendingPasteSelection(newUids);
      // Undo: soft-delete the clones (their edges cascade). pushUndo is a
      // stable useCallback declared below; referenced at call time, so it's
      // assigned by the time paste runs (omitted from deps to avoid a TDZ
      // reference at render).
      pushUndo({ kind: "delete", componentUids: newUids });
      await reload();
    } catch (e) {
      reportError(e);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentParentUid, reload, rf]);

  // Pop the most recent inverse off the CURRENT FOLDER's undo stack and
  // apply it. Errors surface to the error banner — the entry stays popped
  // either way since re-applying a half-failed entry isn't safe without
  // inspection.
  const undo = useCallback(async () => {
    const pid = currentParentUidRef.current;
    const stack = undoStacksByParent.current.get(pid);
    const entry = stack?.pop();
    if (!entry) return;
    try {
      if (entry.kind === "move") {
        if (entry.updates.length === 1) {
          const u = entry.updates[0];
          await updateNode(u.uid, { position: u.position });
        } else if (entry.updates.length > 1) {
          await bulkUpdate(entry.updates);
        }
      } else if (entry.kind === "delete") {
        await bulkDelete({
          componentUids: entry.componentUids,
          edgeUids: entry.edgeUids,
        });
      } else if (entry.kind === "restore") {
        await restoreItems({
          componentUids: entry.componentUids,
          edgeUids: entry.edgeUids,
        });
      }
      await reload();
    } catch (e) {
      reportError(e);
    }
  }, [reload]);

  // Cmd/Ctrl + C / V — window-level so the canvas doesn't need explicit
  // focus. Skipped when focus is in a text editing context so paste-into-
  // input still works normally for the override prompt / palette filter etc.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const cmd = e.metaKey || e.ctrlKey;
      // Cmd/Ctrl+F → component finder. Handled BEFORE the input-focus guard so
      // it overrides the browser's find-in-page even while a field is focused
      // (the finder is more useful here than ctrl-F over the DOM text).
      if (cmd && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFindOpen(true);
        return;
      }
      // Everything below is skipped while editing text so native copy/paste/
      // undo work in inputs (palette filter, override prompt, value editor).
      const ae = document.activeElement;
      if (
        ae &&
        (ae.tagName === "INPUT" ||
          ae.tagName === "TEXTAREA" ||
          (ae as HTMLElement).isContentEditable)
      ) {
        return;
      }
      if (!cmd) return;
      const key = e.key.toLowerCase();
      if (key === "c") {
        e.preventDefault();
        copySelectionToClipboard();
      } else if (key === "v") {
        e.preventDefault();
        void pasteFromClipboard();
      } else if (key === "z" && !e.shiftKey) {
        // Cmd/Ctrl+Z → undo. Cmd/Ctrl+Shift+Z is conventionally redo —
        // not wired yet (would need a redo stack), so we ignore it here so
        // the browser default doesn't fire something unrelated.
        e.preventDefault();
        void undo();
      } else if (key === "d" && e.shiftKey) {
        // Cmd/Ctrl+Shift+D → toggle the click-debug overlay.
        e.preventDefault();
        setClickDebugOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [copySelectionToClipboard, pasteFromClipboard, undo]);

  useEffect(() => {
    reload();
  }, [reload]);

  // Post-navigation focus: when goToComponent set focusAfterLoad and the new
  // view's nodes have populated to include that target, center the viewport
  // on the target and mark it selected. Keeps current zoom — the goal is "I
  // can see where I jumped to", not "fit everything". Cleared once applied
  // so a subsequent unrelated nodes update doesn't re-trigger.
  useEffect(() => {
    if (focusAfterLoad == null) return;
    const targetId = String(focusAfterLoad);
    const target = nodes.find((n) => n.id === targetId);
    if (!target || target.type === "ghost") return;
    // Estimate the node's center. Width is NODE_W; height is the row-count
    // formula from FunctionBlock so we don't have to wait for RF measurement.
    const restComp = useStructural.getState().components.get(focusAfterLoad);
    const userPropCount = restComp
      ? Object.values(restComp.properties).filter(
          (p) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL,
        ).length
      : 4;
    const FB_TITLE = 40;
    const FB_ROW = 18;
    const estH = FB_TITLE + userPropCount * FB_ROW + 4;
    const cx = target.position.x + NODE_W / 2;
    const cy = target.position.y + estH / 2;
    rf.setCenter(cx, cy, { duration: 400, zoom: rf.getZoom() });
    setNodes((ns) =>
      ns.map((n) => {
        const want = n.id === targetId;
        return n.selected === want ? n : { ...n, selected: want };
      }),
    );
    setFocusAfterLoad(null);
  }, [nodes, focusAfterLoad, rf]);

  // Post-paste selection: once the reload after a paste has populated the
  // new uids into nodes, mark exactly those nodes selected and clear
  // everything else. Waits for ALL pasted uids to appear before firing so
  // a partial reload doesn't half-select a subset.
  useEffect(() => {
    if (pendingPasteSelection == null) return;
    const wantedIds = new Set(pendingPasteSelection.map(String));
    let foundCount = 0;
    for (const n of nodes) if (wantedIds.has(n.id)) foundCount++;
    if (foundCount < wantedIds.size) return;
    setNodes((ns) =>
      ns.map((n) => {
        if (n.type === "ghost") return n;
        const want = wantedIds.has(n.id);
        return n.selected === want ? n : { ...n, selected: want };
      }),
    );
    setEdges((es) => es.map((e) => (e.selected ? { ...e, selected: false } : e)));
    setPendingPasteSelection(null);
  }, [nodes, pendingPasteSelection]);

  // Gate: every endpoint of every pending edge must have its handle registered in
  // React Flow's internal store. Handle registration happens in a useEffect inside the
  // Handle component (after mount + measurement); subscribing to `nodeLookup` here
  // re-evaluates on every internal store change, so we flush exactly when ready.
  // Promote parked edges to the live `edges` array as their handles mount.
  // Checked PER EDGE: an edge whose handles can never resolve (e.g. a malformed
  // output→output edge persisted by the engine — its "target" handle is a source
  // handle) is skipped instead of blocking EVERY edge from rendering. Returns a
  // stable string key (the ready edge ids) so the selector only re-renders when
  // the ready SET changes, not on every store tick.
  const readyKey = useRfStore((s) => {
    if (!pendingEdges) return "";
    const lookup = (s as unknown as { nodeLookup: Map<string, unknown> }).nodeLookup;
    if (!lookup) return "";
    const ids: string[] = [];
    for (const e of pendingEdges) {
      const src = lookup.get(e.source) as
        | { internals?: { handleBounds?: { source?: { id?: string | null }[] | null } } }
        | undefined;
      const dst = lookup.get(e.target) as
        | { internals?: { handleBounds?: { target?: { id?: string | null }[] | null } } }
        | undefined;
      const srcBounds = src?.internals?.handleBounds?.source;
      const dstBounds = dst?.internals?.handleBounds?.target;
      if (!srcBounds || !dstBounds) continue;
      if (!srcBounds.some((h) => h.id === e.sourceHandle)) continue;
      if (!dstBounds.some((h) => h.id === e.targetHandle)) continue;
      ids.push(e.id);
    }
    return ids.join(",");
  });
  useEffect(() => {
    if (!pendingEdges) return;
    const ready = new Set(readyKey ? readyKey.split(",") : []);
    setEdges(pendingEdges.filter((e) => ready.has(e.id)));
    if (ready.size === pendingEdges.length) setPendingEdges(null);
  }, [readyKey, pendingEdges]);
  useEffect(() => {
    // Grace period: the ready edges are already live; stop tracking so any
    // still-unresolved (malformed) edges are dropped rather than pinning
    // pendingEdges and re-running the selector on every store change.
    if (!pendingEdges) return;
    const t = window.setTimeout(() => setPendingEdges(null), 1500);
    return () => window.clearTimeout(t);
  }, [pendingEdges]);

  // Available component types grouped by extension (the palette). /api/v0/schema
  // returns each extension's component definitions; the full type string is
  // `<vendor>-<ext>::<name>`. Components are deduped across extension instances.
  useEffect(() => {
    fetch(`${base}/api/v0/schema`)
      .then((r) => r.json())
      .then((j) => {
        const exts = j.data as Array<{
          vendor: string;
          name: string;
          version?: string;
          components?: Array<{ name: string; icon?: string; actions?: ActionDef[] }>;
        }>;
        const seen = new Map<string, PaletteExtension>();
        // Same pass builds the action index: type → its action signatures.
        const actions = new Map<string, ActionDef[]>();
        for (const e of exts) {
          const id = `${e.vendor}-${e.name}`;
          let group = seen.get(id);
          if (!group) {
            group = { id, vendor: e.vendor, name: e.name, version: e.version, components: [] };
            seen.set(id, group);
          }
          const have = new Set(group.components.map((c) => c.type));
          for (const c of e.components ?? []) {
            const type = `${id}::${c.name}`;
            if (c.actions && c.actions.length > 0 && !actions.has(type)) {
              actions.set(type, c.actions);
            }
            if (have.has(type)) continue;
            have.add(type);
            group.components.push({ name: c.name, type, icon: c.icon });
          }
        }
        setActionsByType(actions);
        actionTypesRef.current = new Set(actions.keys());
        // Drop extensions that don't expose any creatable components — they'd render
        // as a dead-end disclosure with nothing inside.
        const list = [...seen.values()].filter((g) => g.components.length > 0);
        list.sort((a, b) => a.id.localeCompare(b.id));
        for (const g of list) g.components.sort((a, b) => a.name.localeCompare(b.name));
        setPalette(list);
      })
      .catch(() => {});
  }, []);

  // Stamp `hasActions` onto already-built nodes when the action index arrives
  // after the first render (build-time stamping in buildRfNodes covers later
  // reloads; this covers the initial schema-load race).
  useEffect(() => {
    const comps = useStructural.getState().components;
    setNodes((ns) => {
      let changed = false;
      const next = ns.map((n) => {
        if (n.type !== "fb") return n;
        const t = comps.get(Number(n.id))?.type;
        const has = t ? actionsByType.has(t) : false;
        if ((n.data as FunctionBlockData).hasActions === has) return n;
        changed = true;
        return { ...n, data: { ...n.data, hasActions: has } };
      });
      return changed ? next : ns;
    });
  }, [actionsByType]);

  // Diagnostics — long-task observer, frame-time percentiles, and the reporter
  // that streams snapshots to the dev sink (POST /__diag). Started once.
  useEffect(() => {
    startDiagnostics();
    startDiagReporter(1000);
    return () => {
      stopDiagReporter();
      stopDiagnostics();
    };
  }, []);

  // Stable callback for VisibilitySub — inline would churn its effect on every
  // App re-render (frequent during a pan as nodes mount/unmount).
  const onVisibleSubscription = useCallback((uids: Set<number>) => {
    wsClient?.setDesiredSubscription(uids);
    diagGauges.subscribedComponents = uids.size;
  }, []);

  // Stable adapter over the module-level wsClient singleton so the DiagPanel's
  // rate controls always hit the live socket (wsClient is null at first render,
  // set later in an effect — a stable adapter reads it lazily each call).
  // Throttle setRate (leading + trailing, 200ms) so nothing can flood the
  // engine with rate changes — historically a setRate stream from the zoom
  // controller crashed it. First call goes out immediately; further calls in
  // the window coalesce and the last one flushes at the end.
  const rateThrottle = useRef<{ timer: number | null; pending: number | null }>({
    timer: null,
    pending: null,
  });
  const wsAdapter = useMemo(
    () => ({
      setRate: (hz: number) => {
        const t = rateThrottle.current;
        if (t.timer != null) {
          t.pending = hz; // in the window — remember the latest, flush on timeout
          return;
        }
        wsClient?.setRate(hz);
        t.pending = null;
        t.timer = window.setTimeout(() => {
          t.timer = null;
          if (t.pending != null) {
            wsClient?.setRate(t.pending);
            t.pending = null;
          }
        }, 200);
      },
      getRate: () => wsClient?.getRate() ?? null,
    }),
    [],
  );

  // Zoom-adaptive push rate. When on, the ZoomRateController scales the WS
  // tick rate to the zoom level (low when zoomed out — you can't read values
  // anyway — full when zoomed in). `rateCeiling` is the upper bound the auto
  // mode scales WITHIN; it's the rate the manual buttons set. Kept SEPARATE
  // from the live wsClient rate (which auto-mode drives down/up) so auto-mode
  // can't ratchet its own ceiling. Both persisted.
  // Default OFF AGAIN. The setRate fix (89b01e0) is INCOMPLETE — re-enabling
  // auto-rate (which streams setRate on zoom) crashed the engine again
  // (API_GAPS #12, reopened): REST went to HTTP 000, confirmed engine-side by
  // an external WS probe that couldn't even get a schema. Until setRate is
  // provably crash-proof under sustained/continuous use, we do NOT auto-send
  // it. Opt-in only.
  const [autoRate, setAutoRate] = useState<boolean>(() => {
    try {
      return window.localStorage.getItem("ce-ui.autoRate") === "1";
    } catch {
      return false;
    }
  });
  // Manual rate (used only when auto-rate is OFF). Persisted. When auto is on,
  // the ZoomRateController owns the rate (zoom → bucket) and the manual value
  // is ignored — no "ceiling" coupling, which is what left it stuck at 5.
  const [manualRate, setManualRate] = useState<number>(() => {
    try {
      const v = Number(window.localStorage.getItem("ce-ui.manualRate"));
      return Number.isFinite(v) && v >= 1 ? v : 10;
    } catch {
      return 10;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.autoRate", autoRate ? "1" : "0");
      window.localStorage.setItem("ce-ui.manualRate", String(manualRate));
    } catch {
      /* ignore */
    }
  }, [autoRate, manualRate]);
  // Manual rate pick (auto OFF): apply it live immediately.
  const onSetManualRate = useCallback((hz: number) => {
    setManualRate(hz);
    wsClient?.setRate(hz);
  }, []);
  // When auto-rate is turned OFF, snap the live rate to the manual value so the
  // session doesn't get stuck at whatever zoom-driven value was last sent.
  useEffect(() => {
    if (!autoRate) wsClient?.setRate(manualRate);
  }, [autoRate, manualRate]);

  // Our display name for presence: an optional user-chosen base (shared across
  // this browser's tabs via localStorage) PLUS a per-tab discriminator so two
  // tabs of the same browser don't show identical names. The discriminator is
  // generated fresh per load (module scope below) — NOT persisted — so it's
  // unique per tab even for a duplicated tab (which copies storage but re-runs
  // module init). Colors already differ per session; this makes names differ
  // too. Click the name in the PresenceBar to set the base.
  const userName = useMemo(() => {
    let base = "user";
    try {
      base = window.localStorage.getItem("ce-ui.userName") || "user";
    } catch {
      /* ignore */
    }
    return `${base}-${TAB_SUFFIX}`;
  }, []);

  // Publish our presence (selection + folder) whenever it changes, debounced
  // so a drag-select sweep doesn't flood the relay. The engine fans this out
  // to other sessions; we never receive our own.
  const selectedUidsKey = nodes
    .filter((n) => n.selected && n.type !== "ghost")
    .map((n) => n.id)
    .join(",");
  // Stable publisher of our current presence — used both by the on-change
  // effect and the heartbeat. Reads the latest selection key via a ref so the
  // heartbeat interval doesn't need to re-subscribe on every selection change.
  const selKeyRef = useRef(selectedUidsKey);
  selKeyRef.current = selectedUidsKey;
  const publishPresence = useCallback(() => {
    const key = selKeyRef.current;
    wsClient?.publishPresence({
      userName,
      selectedComponents: key ? key.split(",").map(Number) : [],
      parentUid: currentParentUid,
    } satisfies PresenceState);
  }, [userName, currentParentUid]);
  useEffect(() => {
    const t = window.setTimeout(publishPresence, 150);
    return () => window.clearTimeout(t);
  }, [selectedUidsKey, currentParentUid, userName, publishPresence]);

  // Heartbeat + TTL sweep. The engine evicts dead sessions only on a slow grace
  // timer, so stale collaborators ("9 others" when there are 2) pile up across
  // reconnects. We republish our presence every HEARTBEAT_MS so live peers stay
  // fresh, and sweep out any collaborator not heard from within PRESENCE_TTL_MS.
  // Dead sessions stop heartbeating → age out; live ones keep refreshing.
  useEffect(() => {
    const HEARTBEAT_MS = 20000;
    const SWEEP_MS = 8000;
    const PRESENCE_TTL_MS = 50000; // ~2.5 missed heartbeats
    const hb = window.setInterval(publishPresence, HEARTBEAT_MS);
    const sw = window.setInterval(() => usePresence.getState().sweep(PRESENCE_TTL_MS), SWEEP_MS);
    return () => {
      window.clearInterval(hb);
      window.clearInterval(sw);
    };
  }, [publishPresence]);

  // Feed the diag gauges from current state on every render — cheap, and keeps
  // the snapshot's structural numbers honest. The rate/timing data is captured
  // continuously inside diagnostics.ts; these are just point-in-time sizes.
  const totalComponentCount = useStructural((s) => s.components.size);
  diagGauges.visibleNodes = nodes.filter((n) => n.type !== "ghost").length;
  diagGauges.ghostNodes = nodes.filter((n) => n.type === "ghost").length;
  diagGauges.edges = edges.length;
  diagGauges.totalComponents = totalComponentCount;
  diagGauges.wsConnected = metrics.wsConnected;
  diagGauges.reconnects = metrics.reconnectCount;
  diagGauges.lastSeq = metrics.lastSeq;

  // WS for live values + schema + topology pushes. Singleton so HMR doesn't reopen.
  useEffect(() => {
    if (wsClient) return;
    const ws = new CeRestWs(wsUrlFromBase(base), {
      onSchema: (msg) => {
        // Slim decode table only: { uid, dataType, statusFlags } per streamable
        // property. Structure comes from REST.
        loadSchemaIndices(msg.properties);
        // Bind the REST client to this session — every mutation now carries
        // `X-CE-Session: <sessionId>` so the engine attributes topology events to us.
        setRestSessionId(msg.sessionId);
        sessionIdRef.current = msg.sessionId;
        // Don't subscribe to everything; the route handler pushes the visible subset
        // via setDesiredSubscription (in reload()).
      },
      onFrame: (frame) => {
        // STATUS sections (typeTag 0x40) carry per-property uint32 status bits,
        // not values — route them to the statusFlags store. Everything else is
        // a typed value section.
        for (const s of frame.sections) {
          if (s.typeTag === TYPE_STATUS) {
            useStatusFlags.getState().applyStatus(s.uids, s.values as ArrayLike<number>);
          } else {
            useValues.getState().apply(s.uids, s.values as ArrayLike<unknown> as never);
          }
        }
      },
      onTopology: (msg) => {
        if (msg.type === "topologyAdded") {
          // Skip the (expensive, scales-with-sheet) reload if we already have
          // everything this event adds — i.e. we appended it optimistically
          // (onAddNode adds the node, onConnect adds the edge). Anything we DON'T
          // have locally — another session, paste, the Connect-to picker, or a
          // cross-folder edge needing a ghost — still reloads to backfill.
          const st = useStructural.getState();
          const haveAll =
            msg.components.every((c) => st.components.has(c.uid)) &&
            msg.edges.every((e) => st.edges.has(e.uid));
          if (haveAll) return;
          scheduleTopologyReload();
        } else if (msg.type === "topologyRemoved") {
          // Splice the removed nodes/edges out of the live RF state without a refetch.
          // Avoids the click-vs-rebuild race that drops in-flight clicks.
          const dropC = new Set(msg.componentUids.map(String));
          const dropE = new Set(msg.edgeUids.map(String));
          setNodes((ns) => ns.filter((n) => !dropC.has(n.id)));
          setEdges((es) => es.filter((e) => !dropE.has(e.id)));
          for (const uid of msg.componentUids) {
            useStructural.getState().removeComponent(uid);
          }
          for (const uid of msg.edgeUids) {
            useStructural.getState().removeEdge(uid);
          }
        } else if (msg.type === "topologyChanged") {
          // If the property SET changed (added or removed) we have to refetch —
          // REST is the source of truth for the structural shape. Otherwise we
          // patch position / name in place to avoid the click-vs-rebuild race.
          const shapeChanged = msg.components.some(
            (c) => (c.addedProperties && c.addedProperties.length > 0) ||
                   (c.removedProperties && c.removedProperties.length > 0) ||
                   c.parent !== undefined,
          );
          if (shapeChanged) {
            scheduleTopologyReload();
            return;
          }
          // Patch in place — DO NOT rebuild the nodes array. Rebuilding would race
          // with the user's clicks: a topology event arriving in the same React batch
          // as a click swallows the click. We only need to update the fields that
          // changed (position, name) and leave everything else (including .selected)
          // alone.
          //
          // Position updates from a DIFFERENT session (another window, the engine
          // itself, etc.) are tweened over POS_ANIM_MS so the node glides instead of
          // jumping. Our own echoes (we just dragged the node and saved it) snap
          // instantly since the local position is already correct.
          const isOwnEcho = msg.originSessionId === sessionIdRef.current;
          const patches = new Map<
            string,
            { position?: { x: number; y: number }; name?: string }
          >();
          for (const p of msg.components) {
            const id = String(p.uid);
            // Our own drag echoes: drop them entirely. The local drag
            // already has the right position via RF's drag state, and even
            // a no-op setNodes outer-array rebuild during an active drag
            // can stutter RF's internal drag handling. Filtering here means
            // setNodes isn't called at all when the whole message is just
            // drag-echo noise for nodes we're currently moving.
            if (isOwnEcho && draggingNodes.current.has(id) && p.position && !p.name) {
              continue;
            }
            patches.set(id, { position: p.position, name: p.name });
          }
          if (patches.size === 0) return;
          setNodes((ns) =>
            ns.map((n) => {
              // Skip ghost sub-nodes — they have no `name` to patch and their
              // layout is derived in reload() (so a real position update on a
              // visible component will rebuild ghost positions on the next
              // reload anyway).
              if (n.type === "ghost") return n;
              const fb = n as RfNode<FunctionBlockData>;
              const p = patches.get(fb.id);
              if (!p) return n;
              const newPos = p.position ?? fb.position;
              const newName = p.name ?? fb.data.name;
              const samePos = newPos === fb.position;
              const sameName = newName === fb.data.name;
              if (samePos && sameName) return n;
              // (Mid-drag own-echoes were filtered out of `patches` above
              // so they never reach this map — setNodes is skipped entirely
              // when the whole message was just drag-echo noise.)
              // Animate the position if it came from another session and we have
              // somewhere to animate from. Leave `position` at its current value;
              // the rAF tick will write interpolated positions until it lands.
              if (!samePos && !isOwnEcho && p.position) {
                animateNodeTo(fb.id, fb.position, p.position);
                return sameName ? n : { ...fb, data: { ...fb.data, name: newName } };
              }
              // Drop any in-flight tween for this node — we're snapping to the
              // authoritative position (own echo, or non-position-only patch).
              posAnims.current.delete(fb.id);
              return {
                ...fb,
                position: samePos ? fb.position : newPos,
                data: sameName ? fb.data : { ...fb.data, name: newName },
              };
            }),
          );
        }
      },
      onPresence: (m) => {
        usePresence.getState().upsert(m.sessionId, (m.state ?? {}) as PresenceState);
      },
      onPresenceSnapshot: (m) => {
        usePresence
          .getState()
          .replaceAll(
            (m.presences ?? []).map((p) => ({
              sessionId: p.sessionId,
              state: (p.state ?? {}) as PresenceState,
            })),
          );
      },
      onPresenceLeft: (m) => {
        usePresence.getState().remove(m.sessionId);
      },
      onOpen: () => {},
      onClose: () => {
        // Connection dropped — clear collaborators so we don't show stale
        // presence while reconnecting. A fresh snapshot arrives on reconnect.
        usePresence.getState().reset();
      },
    });
    ws.connect();
    wsClient = ws;
  }, []);

  // Coalesce topology pushes into one reload per microtask. Different from the engine's
  // per-tick coalescing (which is already at most three messages per tick); this guards
  // against any unmodelled bursts.
  const topoTimer = useRef<number | null>(null);
  const scheduleTopologyReload = useCallback(() => {
    if (topoTimer.current != null) return;
    topoTimer.current = window.setTimeout(() => {
      topoTimer.current = null;
      // Always the latest reload — see `reloadRef` note. Without this, a
      // post-add topology event reloads the root instead of the current folder.
      reloadRef.current();
    }, 0);
  }, []);

  // Latest mouse screen position — paste uses this so Cmd+V drops the
  // clipboard at the cursor instead of offsetting from where the original
  // sat. Updated continuously, no React state churn.
  const mouseScreenPos = useRef<{ x: number; y: number }>({
    x: window.innerWidth / 2,
    y: window.innerHeight / 2,
  });
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      mouseScreenPos.current = { x: e.clientX, y: e.clientY };
    };
    window.addEventListener("mousemove", onMove);
    return () => window.removeEventListener("mousemove", onMove);
  }, []);

  // In-memory clipboard for copy/paste — just the source component UIDs and
  // their on-screen centroid. Paste clones by uid via POST /copy/nodes, so we
  // don't snapshot component/edge data. Single-tab scope. The sources must
  // still exist at paste time (copy clones live components); paste reports an
  // error if they're gone.
  interface ClipboardData {
    uids: number[];
    centroid: { x: number; y: number };
  }
  const clipboardRef = useRef<ClipboardData | null>(null);

  // Undo stacks, keyed by the parent uid of the folder the action was
  // performed in. Switching folders switches which stack Cmd/Ctrl+Z pops
  // from — so an undo never resurrects something the user can't see.
  // Reverting an action across folders would also fight with cross-folder
  // edges and parent-scoped state, so per-folder scoping is the right
  // boundary. Capped at UNDO_MAX entries per folder.
  type UndoEntry =
    // Restore positions for one or more components — pushed on drag-stop,
    // captured at drag-start so the inverse is the pre-drag layout.
    | { kind: "move"; updates: Array<{ uid: number; position: { x: number; y: number } }> }
    // Delete components (and their cascade) — pushed on add (palette, paste)
    // so undo erases what was just created.
    | { kind: "delete"; componentUids?: number[]; edgeUids?: number[] }
    // Restore soft-deleted components/edges by their ORIGINAL uids — pushed
    // when the user deletes. Apply via POST /restore: brings the exact items
    // back with full state (uids, values, the whole deleted subtree). No
    // snapshot reconstruction needed.
    | { kind: "restore"; componentUids?: number[]; edgeUids?: number[] };
  const undoStacksByParent = useRef<Map<number, UndoEntry[]>>(new Map());
  const UNDO_MAX = 50;
  // Mirror the current parent uid into a ref so pushUndo / undo stay
  // stable across folder navigation (no callback identity churn into
  // node data props on every breadcrumb change).
  const currentParentUidRef = useRef(currentParentUid);
  useEffect(() => {
    currentParentUidRef.current = currentParentUid;
  }, [currentParentUid]);
  const pushUndo = useCallback((entry: UndoEntry) => {
    const pid = currentParentUidRef.current;
    const m = undoStacksByParent.current;
    let stack = m.get(pid);
    if (!stack) {
      stack = [];
      m.set(pid, stack);
    }
    stack.push(entry);
    if (stack.length > UNDO_MAX) stack.shift();
  }, []);

  // Set of node ids currently mid-drag. The position-animation rAF and the
  // topology-echo handler check this to skip applying our own position
  // echoes back onto a node the user is actively moving — otherwise the
  // echo (which lags the cursor by a network round-trip) would briefly
  // snap the node backwards on every PATCH response.
  const draggingNodes = useRef(new Set<string>());

  // Throttled position PATCH during drag. The user expects other sessions to
  // see the component moving in near-real-time, not just on drop. 100ms →
  // ~10 Hz updates, smoothed by the receiver's exponential ease.
  //
  // Single throttle window across the whole drag group: when 8 components
  // move together, we want ONE PATCH /bulknodes per tick carrying all 8
  // positions, not 8 separate /nodes/uid/{uid} PATCHes per tick. Engine
  // load drops by ~Nx for multi-select moves, and own-echo broadcasts
  // arrive as a single topology event for the whole group.
  const DRAG_PATCH_MS = 100;
  const dragPatchState = useRef<{
    lastSent: number;
    pending: Map<number, { x: number; y: number }>;
    timer: number | null;
  }>({ lastSent: 0, pending: new Map(), timer: null });

  const flushDragPatch = useCallback(() => {
    const s = dragPatchState.current;
    s.timer = null;
    if (s.pending.size === 0) return;
    s.lastSent = performance.now();
    const updates = [...s.pending.entries()].map(([uid, p]) => ({
      uid,
      position: { x: Math.round(p.x), y: Math.round(p.y) },
    }));
    s.pending.clear();
    // Single-uid: single PATCH (cheaper round-trip than wrapping in
    // bulknodes). Multi-uid: one bulk call.
    if (updates.length === 1) {
      const u = updates[0];
      updateNode(u.uid, { position: u.position }).catch(() => {});
    } else {
      bulkUpdate(updates).catch(() => {
        /* mid-drag bulk errors are silent — drag-stop will surface
           persistent failures via its own PATCHes. */
      });
    }
  }, []);

  const sendDragPatch = useCallback(
    (uid: number, pos: { x: number; y: number }) => {
      const s = dragPatchState.current;
      // Coalesce: the LATEST position for any uid in this throttle window
      // wins. Map.set replaces, so only one entry per uid lands in the next
      // flush regardless of how many onNodeDrag callbacks fired in between.
      s.pending.set(uid, pos);
      const now = performance.now();
      if (now - s.lastSent >= DRAG_PATCH_MS) {
        flushDragPatch();
        return;
      }
      if (s.timer == null) {
        s.timer = window.setTimeout(flushDragPatch, DRAG_PATCH_MS - (now - s.lastSent));
      }
    },
    [flushDragPatch],
  );

  const cancelDragPatch = useCallback((id: string) => {
    const s = dragPatchState.current;
    s.pending.delete(Number(id));
    if (s.timer != null && s.pending.size === 0) {
      window.clearTimeout(s.timer);
      s.timer = null;
    }
  }, []);

  // Forward RF-internal edge changes (selection, removal) into our controlled
  // edge state. We don't need to filter — `selected` toggles from box-selection
  // and explicit selection emits from our document-level handler both come
  // through here, and applyEdgeChanges is idempotent on equal updates.
  const onEdgesChange = useCallback((changes: EdgeChange<RfEdge>[]) => {
    // Drop RF's own `select` changes — the document-level pointer handler owns
    // selection (nodes AND edges), so applying RF's too would fight it. Keep
    // everything else (remove, etc.). Elements stay interactive (clickable).
    setEdges((es) => applyEdgeChanges(changes.filter((c) => c.type !== "select"), es));
  }, []);

  // Right-click on an edge → context menu (Reevaluate, Delete).
  const [edgeMenu, setEdgeMenu] = useState<{ x: number; y: number; edgeId: string } | null>(
    null,
  );
  const onEdgeContextMenu = useCallback((e: React.MouseEvent, edge: RfEdge) => {
    e.preventDefault();
    e.stopPropagation();
    setEdgeMenu({ x: e.clientX, y: e.clientY, edgeId: edge.id });
    // Make sure the right-clicked edge is selected so the menu acts on a
    // visible target.
    setEdges((es) =>
      es.map((ed) => (ed.id === edge.id ? (ed.selected ? ed : { ...ed, selected: true }) : ed)),
    );
  }, []);

  // There's no bulk edge-update endpoint (PATCH /bulknodes is component-only —
  // API_GAPS #16), so multi-edge actions still issue one PATCH /edge/uid/{uid}
  // per edge. But fire them CONCURRENTLY (Promise.all) rather than sequentially
  // awaiting — wall time drops from N×RTT to ~1×RTT.
  const reEvaluateEdges = useCallback(async (ids: number[]) => {
    const results = await Promise.allSettled(
      ids.map((uid) => restUpdateEdge(uid, { reEvaluate: true })),
    );
    const failed = results.find((r) => r.status === "rejected") as PromiseRejectedResult | undefined;
    if (failed) reportError(failed.reason);
  }, []);

  // Promote edges to loopback. Per the OpenAPI: `loopBack` may only be set to
  // `true` — once an edge is marked loopback, the engine never clears it
  // automatically and there's no API to clear it either. The only way out is
  // to delete the edge. Patch the dotted-grey loopback style in place for the
  // edges that succeeded — no full reload just to repaint a few edges.
  const setEdgesLoopBack = useCallback(async (ids: number[]) => {
    if (ids.length === 0) return;
    const results = await Promise.allSettled(
      ids.map((uid) => restUpdateEdge(uid, { loopBack: true })),
    );
    const failed = results.find((r) => r.status === "rejected") as PromiseRejectedResult | undefined;
    if (failed) reportError(failed.reason);
    const ok = ids.filter((_, i) => results[i].status === "fulfilled");
    if (ok.length === 0) return;
    const okSet = new Set(ok.map(String));
    const st = useStructural.getState();
    for (const uid of ok) {
      const e = st.edges.get(uid);
      if (e) st.upsertEdge({ ...e, loopBack: true });
    }
    setEdges((es) =>
      es.map((e) =>
        okSet.has(e.id)
          ? { ...e, style: { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } }
          : e,
      ),
    );
  }, []);

  // Drag-to-move → PATCH on drag-stop only. Also drag-along any ghost
  // sub-nodes anchored to the moving component so cross-folder edge stubs
  // stay attached as the parent moves.
  const onNodesChange = useCallback((changes: NodeChange<AnyNode>[]) => {
    setNodes((ns) => {
      // Drop RF's own `select` changes — the document-level pointer handler owns
      // selection. Without this, RF and our handler both toggle and race (that's
      // what made shift-click take several attempts). Everything else (position,
      // dimensions, etc.) still applies.
      const next = applyNodeChanges(changes.filter((c) => c.type !== "select"), ns);
      // Collect new positions of anchor components from this batch so we can
      // recompute the positions of their ghosts in a single pass.
      const movedAnchors = new Map<string, { x: number; y: number }>();
      for (const ch of changes) {
        if (ch.type !== "position" || !ch.position) continue;
        // Only real components are anchors. Ghosts moving themselves (they
        // can't — they're non-draggable) wouldn't be anchors anyway.
        const n = next.find((m) => m.id === ch.id);
        if (!n || n.type === "ghost") continue;
        movedAnchors.set(ch.id, ch.position);
      }
      if (movedAnchors.size === 0) return next;
      return next.map((n) => {
        if (n.type !== "ghost") return n;
        const g = n as RfNode<GhostNodeData>;
        const anchor = movedAnchors.get(String(g.data.anchorUid));
        if (!anchor) return n;
        const gx =
          g.data.side === "input"
            ? anchor.x + NODE_W + GHOST_GAP
            : anchor.x - g.data.width - GHOST_GAP;
        const gy = anchor.y + FB_TITLE_H + g.data.anchorRowIdx * FB_ROW_H;
        return { ...g, position: { x: gx, y: gy } };
      });
    });
    // Debug: capture only select changes — what we're chasing right now. Resolve to
    // component name (instead of UID) so the click-debugger row is readable.
    const selChanges = changes.filter((c) => c.type === "select");
    if (selChanges.length > 0) {
      const comps = useStructural.getState().components;
      const compact = selChanges
        .map((c) => {
          const id = (c as { id: string }).id;
          const sel = (c as { selected: boolean }).selected;
          const name = comps.get(Number(id))?.name ?? id;
          return `${name}=${sel ? "+" : "-"}`;
        })
        .join(" ");
      metrics.lastSelChange = compact;
      metrics.lastSelChangeAt = performance.now();
    }
    for (const ch of changes) {
      if (ch.type === "position" && ch.dragging) {
        // User is interacting with this node — cancel any in-flight tween
        // for it so we don't fight the drag. Drag streaming itself is
        // handled by onNodeDragStart/Drag/Stop below: ch.dragging in
        // NodeChange is unreliable across the first frame of certain RF
        // configurations, so we rely on the explicit drag callbacks
        // instead for the draggingNodes set and the PATCH stream.
        posAnims.current.delete(ch.id);
      }
    }
  }, []);

  // Pre-drag positions, captured at drag-start so the drag-stop handler
  // can push an undo entry that restores the original layout.
  const dragStartPositions = useRef<Array<{ uid: number; position: { x: number; y: number } }>>(
    [],
  );

  // Explicit drag lifecycle from React Flow — used to maintain draggingNodes
  // and stream throttled position PATCHes. Replaces reading the unreliable
  // `dragging` flag off NodeChange entries.
  const onNodeDragStart = useCallback(
    (_e: unknown, _node: AnyNode, ns: AnyNode[]) => {
      const real = ns.filter((n) => n.type !== "ghost");
      for (const n of real) draggingNodes.current.add(n.id);
      // Snapshot the pre-drag positions for the undo stack. Drag-stop reads
      // this and pushes a "move back" inverse before clearing.
      dragStartPositions.current = real.map((n) => ({
        uid: Number(n.id),
        position: { x: Math.round(n.position.x), y: Math.round(n.position.y) },
      }));
    },
    [],
  );
  const onNodeDrag = useCallback(
    (_e: unknown, _node: AnyNode, ns: AnyNode[]) => {
      for (const n of ns) {
        if (n.type === "ghost") continue;
        sendDragPatch(Number(n.id), n.position);
      }
    },
    [sendDragPatch],
  );
  const onNodeDragStop = useCallback(
    (_e: unknown, _node: AnyNode, ns: AnyNode[]) => {
      // Clear the drag flag(s) BEFORE the final PATCH so the topology echo
      // from this last call applies cleanly (no own-echo suppression).
      const real = ns.filter((n) => n.type !== "ghost");
      for (const n of real) {
        draggingNodes.current.delete(n.id);
        cancelDragPatch(n.id);
      }
      if (real.length === 0) return;
      const updates = real.map((n) => ({
        uid: Number(n.id),
        position: { x: Math.round(n.position.x), y: Math.round(n.position.y) },
      }));
      // Push undo BEFORE the network call so a failed PATCH still leaves the
      // user with a way back. Filter out no-op drags (drag started, dragged,
      // released without moving) so the undo stack doesn't pile up empties.
      const starts = dragStartPositions.current;
      if (starts.length > 0) {
        const moved = starts.filter((s) => {
          const u = updates.find((x) => x.uid === s.uid);
          return u && (u.position.x !== s.position.x || u.position.y !== s.position.y);
        });
        if (moved.length > 0) pushUndo({ kind: "move", updates: moved });
      }
      dragStartPositions.current = [];
      // Match the streaming path: single → /nodes/uid/{uid}, multi → /bulknodes.
      if (updates.length === 1) {
        const u = updates[0];
        updateNode(u.uid, { position: u.position }).catch((e) =>
          reportError(e),
        );
      } else {
        bulkUpdate(updates).catch((e) => reportError(e));
      }
    },
    [cancelDragPatch, pushUndo],
  );

  // Connect — drag from a source handle (output) to a target handle (input). Uses the
  // All node-click selection is handled by the document-level pointer capture
  // listener above. React Flow's own onNodeClick / onPaneClick don't fire reliably
  // when `selectionOnDrag` is enabled, so we bypass them.

  // Connect — drag a source (output) handle to a target (input) handle. Handle IDs
  // are property UIDs.
  const onConnect = useCallback(async (c: Connection) => {
    if (!c.source || !c.target || !c.sourceHandle || !c.targetHandle) return;
    try {
      // If a handle is an exposed port, the prop belongs to the off-canvas CHILD
      // component, not the folder node the handle is drawn on — store the edge
      // against the child (the visual still attaches to the folder's port).
      const remap = exposedRemapRef.current;
      const srcUid = remap.get(Number(c.sourceHandle)) ?? Number(c.source);
      const tgtUid = remap.get(Number(c.targetHandle)) ?? Number(c.target);
      const created = await restAddEdge({
        sourceUid: srcUid,
        sourcePropUid: Number(c.sourceHandle),
        targetUid: tgtUid,
        targetPropUid: Number(c.targetHandle),
      });
      if (created?.uid != null) {
        // Fast path: append just this edge instead of reloading the whole sheet.
        // onConnect only fires for a drag between two VISIBLE handles, so both
        // endpoints are on-canvas — the edge is in-folder, no ghost needed. The
        // WS topologyAdded echo is skipped because the store already has it.
        useStructural.getState().upsertEdge(created);
        const isLoop = created.loopBack === true;
        const rfEdge: RfEdge = {
          id: String(created.uid),
          source: c.source,
          sourceHandle: c.sourceHandle,
          target: c.target,
          targetHandle: c.targetHandle,
          style: isLoop
            ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" }
            : { stroke: "#4a9eff", strokeWidth: 1.5 },
          animated: false,
        };
        setEdges((es) => (es.some((e) => e.id === rfEdge.id) ? es : [...es, rfEdge]));
      } else {
        await reload(); // unexpected: no edge returned — fall back
      }
    } catch (e) {
      reportError(e);
    }
  }, [reload]);

  // Delete keys: remove selected nodes & edges.
  // Delete real components in one bulk call when there's more than one;
  // ghosts are derived from edges and skipped. Single-component delete
  // still uses /nodes/uid/{uid} since the round-trip is slightly cheaper
  // than wrapping a one-entry batch.
  //
  // Delete is soft-delete, so undo is just `restore` by the deleted uids —
  // no pre-delete snapshot, and it correctly brings back a folder's children
  // (the engine cascades the soft-delete and restore reverses the whole set).
  const onNodesDelete = useCallback(
    async (ns: AnyNode[]) => {
      const real = ns.filter((n) => n.type !== "ghost");
      if (real.length === 0) return;
      const uids = real.map((n) => Number(n.id));
      try {
        if (uids.length === 1) {
          await restRemoveNode(uids[0]);
        } else {
          await bulkDelete({ componentUids: uids });
        }
        for (const uid of uids) useStructural.getState().removeComponent(uid);
        pushUndo({ kind: "restore", componentUids: uids });
      } catch (e) {
        reportError(e);
      }
      await reload();
    },
    [reload, pushUndo],
  );

  const onEdgesDelete = useCallback(
    async (es: RfEdge[]) => {
      if (es.length === 0) return;
      const uids = es.map((e) => Number(e.id));
      try {
        if (uids.length === 1) {
          await restRemoveEdge(uids[0]);
        } else {
          await bulkDelete({ edgeUids: uids });
        }
        for (const uid of uids) useStructural.getState().removeEdge(uid);
        pushUndo({ kind: "restore", edgeUids: uids });
      } catch (err) {
        reportError(err);
      }
      setEdges((cur) => cur.filter((e) => !es.find((d) => d.id === e.id)));
    },
    [pushUndo],
  );

  const onAddNode = useCallback(
    async (type: string, worldPos?: { x: number; y: number }) => {
      const vp = rf.getViewport();
      const pos =
        worldPos ??
        {
          x: Math.round((window.innerWidth / 2 - vp.x) / vp.zoom),
          y: Math.round((window.innerHeight / 2 - vp.y) / vp.zoom),
        };
      // The engine validates names against a strict charset and the auto-derived default
      // can include `::` from the type → rejected with "Name contains invalid characters".
      // Derive a clean base from the type's local segment and find the first free suffix
      // under the current parent.
      const base = sanitizeName(type);
      const siblings = new Set(
        Array.from(useStructural.getState().components.values())
          .filter((c) => c.parent === currentParentUid)
          .map((c) => c.name),
      );
      let name = base;
      let n = 1;
      while (siblings.has(name)) {
        n += 1;
        name = `${base}${n}`;
      }
      try {
        const created = await restAddNode({
          type,
          name,
          parentUid: currentParentUid,
          defaultValues: { position: { x: Math.round(pos.x), y: Math.round(pos.y) } },
        });
        if (created?.uid != null) {
          pushUndo({ kind: "delete", componentUids: [created.uid] });
          // Fast path: append just this node instead of reloading the whole
          // sheet (a full reload re-fetches + rebuilds every node, so on a large
          // sheet it's the add lag spike). restAddNode returns the full
          // component, so no extra fetch is needed; the WS topologyAdded echo
          // for our own session is suppressed (see onTopology).
          useStructural.getState().upsertComponent(created);
          const [rfNode] = buildRfNodes(
            [created],
            enter,
            openNodeContextMenu,
            undefined,
            actionTypesRef.current,
          );
          if (rfNode) {
            setNodes((ns) => (ns.some((n) => n.id === rfNode.id) ? ns : [...ns, rfNode]));
          }
        } else {
          await reload(); // unexpected: no component returned — fall back
        }
      } catch (e) {
        reportError(e);
      }
    },
    [rf, reload, currentParentUid, pushUndo, enter, openNodeContextMenu],
  );

  // Creatable component types for the ConnectPicker's "New" flow.
  const componentTypes = useMemo(
    () =>
      palette.flatMap((g) =>
        g.components.map((c) => ({ name: c.name, type: c.type, group: g.id })),
      ),
    [palette],
  );

  // Create one component of `type` in the current folder and return it (with its
  // properties) so the caller can wire up to it. Mirrors onAddNode but returns
  // the created Component instead of being fire-and-forget.
  const createComponent = useCallback(
    async (
      type: string,
      opts?: { nearUid?: number; side?: "left" | "right" },
    ): Promise<Component | null> => {
      const baseName = sanitizeName(type);
      const siblings = new Set(
        Array.from(useStructural.getState().components.values())
          .filter((c) => c.parent === currentParentUid)
          .map((c) => c.name),
      );
      let name = baseName;
      let n = 1;
      while (siblings.has(name)) {
        n += 1;
        name = `${baseName}${n}`;
      }
      // Place next to the source component when the picker passes one — to its
      // right for an output→input link, to its left for an input←output link —
      // so the new node lands beside it, not at screen center. Fall back to the
      // viewport center when there's no anchor.
      const near =
        opts?.nearUid != null
          ? useStructural.getState().components.get(opts.nearUid)
          : undefined;
      let pos: { x: number; y: number };
      if (near?.metadata?.position) {
        const GAP = 80;
        const dx = (NODE_W + GAP) * (opts?.side === "left" ? -1 : 1);
        pos = { x: near.metadata.position.x + dx, y: near.metadata.position.y };
      } else {
        const vp = rf.getViewport();
        pos = {
          x: Math.round((window.innerWidth / 2 - vp.x) / vp.zoom),
          y: Math.round((window.innerHeight / 2 - vp.y) / vp.zoom),
        };
      }
      try {
        const created = await restAddNode({
          type,
          name,
          parentUid: currentParentUid,
          defaultValues: { position: { x: Math.round(pos.x), y: Math.round(pos.y) } },
        });
        if (created?.uid != null) {
          pushUndo({ kind: "delete", componentUids: [created.uid] });
          // Incremental append — same fast path as onAddNode, no full reload.
          useStructural.getState().upsertComponent(created);
          const [rfNode] = buildRfNodes(
            [created],
            enter,
            openNodeContextMenu,
            undefined,
            actionTypesRef.current,
          );
          if (rfNode) setNodes((ns) => (ns.some((n) => n.id === rfNode.id) ? ns : [...ns, rfNode]));
        }
        return created ?? null;
      } catch (e) {
        reportError(e);
        return null;
      }
    },
    [rf, currentParentUid, pushUndo, enter, openNodeContextMenu],
  );

  // Edge add for the Connect-to picker. Appends the edge if both endpoints are
  // in the current view (in-folder); falls back to a reload only when the target
  // is in another folder (needs a ghost). Keeps connect-to-existing AND
  // connect-to-new fast instead of full-reloading.
  const connectEdge = useCallback(
    async (payload: {
      sourceUid: number;
      sourcePropUid: number;
      targetUid: number;
      targetPropUid: number;
    }) => {
      const created = await restAddEdge(payload);
      if (created?.uid == null) return;
      useStructural.getState().upsertEdge(created);
      const st = useStructural.getState();
      const inView = st.components.has(payload.sourceUid) && st.components.has(payload.targetUid);
      if (inView) {
        const isLoop = created.loopBack === true;
        const rfEdge: RfEdge = {
          id: String(created.uid),
          source: String(payload.sourceUid),
          sourceHandle: String(payload.sourcePropUid),
          target: String(payload.targetUid),
          targetHandle: String(payload.targetPropUid),
          style: isLoop
            ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" }
            : { stroke: "#4a9eff", strokeWidth: 1.5 },
          animated: false,
        };
        setEdges((es) => (es.some((e) => e.id === rfEdge.id) ? es : [...es, rfEdge]));
      } else {
        await reload(); // cross-folder target → needs a ghost
      }
    },
    [reload],
  );

  // Expose a child's prop as a port on the current container (folder). Writes the
  // container's __facets (read-modify-write of the freshly-fetched value, since
  // the container itself is off-canvas one level up), then reloads.
  const exposeProp = useCallback(
    async (
      childPropUid: number,
      childComponentUid: number,
      side: "input" | "output",
      defaultLabel: string,
    ) => {
      const parentUid = currentParentUid;
      try {
        const resp = await getNodeByUid(parentUid, { depth: 0 });
        const parent = resp.nodes[0];
        const facet = parseFacet(rawFacet(parent?.properties) ?? "");
        // Record the child's __facets prop uid so we can subscribe to it and read
        // the child prop's label/unit/aliases LIVE (no stale copy). The child is
        // visible here, so grab the uid from the store.
        const child = useStructural.getState().components.get(childComponentUid);
        const facetPropUid = child?.properties?.[FACET_PROP]?.uid;
        const existing = facet.get(childPropUid) ?? {};
        facet.set(childPropUid, {
          ...existing,
          expose: side,
          childComponent: childComponentUid,
          facetProp: facetPropUid,
          // Fallback display name only — live label/unit/aliases come from the
          // child's streamed __facets.
          label: existing.label ?? defaultLabel,
        });
        await updateNode(parentUid, {
          properties: { [FACET_PROP]: { value: serializeFacet(facet) } },
        });
        await reload();
      } catch (e) {
        reportError(e);
      }
    },
    [currentParentUid, reload],
  );

  // Remove an exposed port: drop its record from the folder's __facets. The
  // folder is a visible node here, but fetch it fresh to read-modify-write safely.
  const unexposeProp = useCallback(
    async (folderUid: number, childPropUid: number) => {
      try {
        const resp = await getNodeByUid(folderUid, { depth: 0 });
        const folder = resp.nodes[0];
        const facet = parseFacet(rawFacet(folder?.properties) ?? "");
        facet.delete(childPropUid);
        await updateNode(folderUid, {
          properties: { [FACET_PROP]: { value: serializeFacet(facet) } },
        });
        await reload();
      } catch (e) {
        reportError(e);
      }
    },
    [reload, reportError],
  );

  // Open Details for any component. If it's off-canvas (e.g. the child behind an
  // exposed port), fetch it into the store first so the panel has its props/facet.
  const openDetails = useCallback(async (componentUid: number) => {
    if (!useStructural.getState().components.has(componentUid)) {
      try {
        const resp = await getNodeByUid(componentUid, { depth: 0 });
        const c = resp.nodes[0];
        if (c) useStructural.getState().upsertComponent(c);
      } catch {
        /* fall through — panel shows "no editable properties" */
      }
    }
    setDetailsUid(componentUid);
  }, []);

  const ceCtx = useMemo(
    () => ({
      componentTypes,
      createComponent,
      connectEdge,
      exposeProp,
      unexposeProp,
      openDetails,
      parentName: crumbs.length > 1 ? crumbs[crumbs.length - 1]?.name : undefined,
    }),
    [componentTypes, createComponent, connectEdge, exposeProp, unexposeProp, openDetails, crumbs],
  );

  // DnD: dragging a palette item into the canvas drops a new component at the cursor.
  const onDragOver = useCallback((e: React.DragEvent) => {
    if (e.dataTransfer.types.includes(DND_TYPE)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }, []);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      const type = e.dataTransfer.getData(DND_TYPE);
      if (!type) return;
      e.preventDefault();
      const worldPos = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      onAddNode(type, worldPos);
    },
    [rf, onAddNode],
  );

  return (
    <CeWiresheetContext.Provider value={ceCtx}>
      <style>{EDGE_SELECTED_CSS}</style>
      <div
        style={{ position: "absolute", inset: 0 }}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onPointerDown={onCanvasPointerDown}
        onPointerMove={onCanvasPointerMove}
        onPointerUp={onCanvasPointerUp}
        onContextMenu={(e) => {
          // Only suppress the native browser menu. Our pane menu opens on
          // pointer-UP (see onCanvasPointerUp) — the browser fires this on PRESS,
          // before we can tell a click from a drag-select.
          e.preventDefault();
        }}
      >
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onEdgeContextMenu={onEdgeContextMenu}
          onNodeDragStart={onNodeDragStart}
          onNodeDrag={onNodeDrag}
          onNodeDragStop={onNodeDragStop}
          onConnect={onConnect}
          onNodesDelete={onNodesDelete}
          onEdgesDelete={onEdgesDelete}
          defaultViewport={{ x: 80, y: 80, zoom: 1 }}
          minZoom={0.1}
          maxZoom={2}
          // Cull off-screen nodes/edges from the React render tree. At
          // 100+ components the whole view rendering every node — even
          // those panned offscreen — is the main "feels frozen" cause.
          onlyRenderVisibleElements
          // Skip RF's measurement pass when a node has no width/height set
          // yet. Cheap on initial render with many nodes.
          nodeOrigin={[0, 0]}
          deleteKeyCode={["Delete", "Backspace"]}
          // Mouse: LEFT-drag pans the pane; RIGHT-drag marquee-selects (handled
          // by the wrapper's pointer handlers + getIntersectingNodes, since RF's
          // selectionOnDrag is left-button-only).
          panOnDrag={[0]}
          selectionMode={SelectionMode.Partial}
          multiSelectionKeyCode={["Shift", "Meta", "Control"]}
          // Disable RF's own Shift rubber-band: its default selectionKeyCode is
          // "Shift", which puts RF into box-select mode on Shift-press and lets
          // its d3 layer swallow the pointer events. We marquee-select via the
          // custom right-drag instead.
          selectionKeyCode={null}
          // NB: elements stay selectable (interactive) so edges/nodes remain
          // clickable; we instead drop RF's `select` changes in onNodes/Edges
          // Change so the document-level handler is the single selection
          // authority. (elementsSelectable={false} would also kill edge
          // pointer-events → unclickable edges.)
          // Treat any mouse movement under 4px as a click — fixes occasional missed
          // selects when the cursor wobbles a pixel between mousedown and mouseup.
          nodeDragThreshold={4}
          // Wheel still scrolls/zooms; that doesn't change.
          panOnScroll={false}
          panOnScrollMode={PanOnScrollMode.Free}
          proOptions={{ hideAttribution: true }}
        >
          <Background color="#1f242e" gap={20} />
          {/* Overview map, bottom-right. Dots colored by kind so the layout is
              readable at a glance: folders (have children) accent-blue, ghost
              cross-folder stubs dim, plain components gray. Click/drag the map
              to jump the viewport. */}
          <MiniMap
            position="bottom-right"
            pannable
            zoomable
            ariaLabel="Graph overview"
            style={{
              backgroundColor: "#2b313c",
              border: "1px solid #3d444d",
              borderRadius: 6,
            }}
            maskColor="rgba(10,12,16,0.45)"
            nodeStrokeWidth={2}
            nodeColor={miniMapNodeColor}
            nodeStrokeColor={miniMapNodeStroke}
          />
          <ZoomRateController enabled={autoRate} setRate={wsAdapter.setRate} />
          <VisibilitySub onVisible={onVisibleSubscription} />
        </ReactFlow>
      </div>
      <Palette
        palette={palette}
        onAdd={(t) => onAddNode(t)}
        currentParentUid={currentParentUid}
      />
      <Breadcrumb crumbs={crumbs} onGoTo={goToCrumb} />
      {clickDebugOpen && <ClickDebugger />}
      <EventsPanel />
      <DiagPanel
        wsRef={wsAdapter}
        autoRate={autoRate}
        manualRate={manualRate}
        onSetManualRate={onSetManualRate}
        onToggleAutoRate={() => setAutoRate((v) => !v)}
      />
      <PresenceBar />
      <FindPanel
        open={findOpen}
        currentParentUid={currentParentUid}
        onClose={() => setFindOpen(false)}
        onPick={(uid) => void goToComponent(uid)}
      />
      {marqueeRect && (
        <div
          style={{
            position: "fixed",
            left: marqueeRect.x,
            top: marqueeRect.y,
            width: marqueeRect.w,
            height: marqueeRect.h,
            border: "1px solid #4a9eff",
            background: "rgba(74,158,255,0.12)",
            zIndex: 40,
            pointerEvents: "none",
          }}
        />
      )}
      {nodeMenu && !movePickerOpen && !actionPickerOpen && detailsUid === null && (
        <NodeContextMenu
          x={nodeMenu.x}
          y={nodeMenu.y}
          hasActions={
            getActionsFor(nodes.filter((n) => n.selected).map((n) => Number(n.id))).length > 0
          }
          canRename={nodes.filter((n) => n.selected).length === 1}
          count={nodes.filter((n) => n.selected).length}
          uid={
            nodes.filter((n) => n.selected).length === 1
              ? Number(nodes.filter((n) => n.selected)[0].id)
              : undefined
          }
          name={
            nodes.filter((n) => n.selected).length === 1
              ? useStructural.getState().components.get(Number(nodes.filter((n) => n.selected)[0].id))
                  ?.name
              : undefined
          }
          onRename={async () => {
            const sel = nodes.filter((n) => n.selected).map((n) => Number(n.id));
            setNodeMenu(null);
            if (sel.length !== 1) return;
            const uid = sel[0];
            const cur = useStructural.getState().components.get(uid);
            const next = window.prompt("Rename component", cur?.name ?? "");
            if (next == null) return;
            const trimmed = next.trim();
            if (!trimmed || trimmed === cur?.name) return;
            try {
              await updateNode(uid, { name: trimmed });
              await reload();
            } catch (e) {
              reportError(e);
            }
          }}
          onDetails={() => {
            const sel = nodes.filter((n) => n.selected).map((n) => Number(n.id));
            if (sel.length === 1) setDetailsUid(sel[0]);
          }}
          onMoveInto={() => setMovePickerOpen(true)}
          onAction={() => setActionPickerOpen(true)}
          onClose={() => setNodeMenu(null)}
        />
      )}
      {nodeMenu && actionPickerOpen && (
        <ActionPicker
          x={nodeMenu.x}
          y={nodeMenu.y}
          targetUids={nodes.filter((n) => n.selected).map((n) => Number(n.id))}
          actions={getActionsFor(nodes.filter((n) => n.selected).map((n) => Number(n.id)))}
          onInvoke={invokeAction}
          onClose={() => {
            setActionPickerOpen(false);
            setNodeMenu(null);
          }}
        />
      )}
      {nodeMenu && movePickerOpen && (
        <MoveIntoPicker
          x={nodeMenu.x}
          y={nodeMenu.y}
          // Move every selected node (the right-click handler already ensured
          // the right-clicked node is in the selection). Drop the moving nodes
          // themselves from the candidate list — can't reparent into self.
          movingUids={
            nodes.filter((n) => n.selected).map((n) => Number(n.id))
          }
          onMove={async (newParent) => {
            const moving = nodes.filter((n) => n.selected).map((n) => Number(n.id));
            for (const uid of moving) {
              try {
                await updateNode(uid, { parentUid: newParent });
              } catch (e) {
                reportError(e);
              }
            }
            setMovePickerOpen(false);
            setNodeMenu(null);
            // Reload — the moved components leave the current view (or stay if
            // we moved into a sibling of their old parent... no, sibling moves
            // also exit the view since we render only direct children).
            await reload();
          }}
          onClose={() => {
            setMovePickerOpen(false);
            setNodeMenu(null);
          }}
        />
      )}
      {detailsUid != null && (
        <DetailsPanel
          componentUid={detailsUid}
          onSave={async (facetString) => {
            try {
              await updateNode(detailsUid, {
                properties: { [FACET_PROP]: { value: facetString } },
              });
              await reload();
            } catch (e) {
              reportError(e);
            }
          }}
          onClose={() => {
            setDetailsUid(null);
            setNodeMenu(null);
          }}
        />
      )}
      {paneMenu && (
        <PaneContextMenu
          x={paneMenu.x}
          y={paneMenu.y}
          canGoUp={crumbs.length > 1}
          parentName={crumbs.length > 1 ? crumbs[crumbs.length - 2].name : ""}
          palette={palette}
          canPaste={(clipboardRef.current?.uids.length ?? 0) > 0}
          onUp={() => goToCrumb(crumbs.length - 2)}
          onAdd={(type) =>
            void onAddNode(type, rf.screenToFlowPosition({ x: paneMenu.x, y: paneMenu.y }))
          }
          onPaste={() => {
            mouseScreenPos.current = { x: paneMenu.x, y: paneMenu.y };
            void pasteFromClipboard();
          }}
          onClose={() => setPaneMenu(null)}
        />
      )}
      {edgeMenu && (() => {
        // Look up loopback state from REST (source of truth). Determines which
        // primary action the menu offers. If the right-clicked edge can't be
        // found (e.g. just got removed under us), suppress the menu entirely.
        const rest = useStructural.getState().edges.get(Number(edgeMenu.edgeId));
        if (!rest) return null;
        const isLoop = rest.loopBack === true;
        return (
          <EdgeContextMenu
            x={edgeMenu.x}
            y={edgeMenu.y}
            isLoopBack={isLoop}
            onPrimary={() => {
              const ids = selectedEdgeIds(edges, edgeMenu.edgeId);
              // Filter selection to the same kind as the right-clicked edge so
              // a mixed selection doesn't accidentally apply the wrong action.
              const filtered = ids.filter((id) => {
                const e = useStructural.getState().edges.get(Number(id));
                return e ? (e.loopBack === true) === isLoop : false;
              });
              if (isLoop) void reEvaluateEdges(filtered.map(Number));
              else void setEdgesLoopBack(filtered.map(Number));
              setEdgeMenu(null);
            }}
            onDelete={() => {
              const ids = selectedEdgeIds(edges, edgeMenu.edgeId);
              const drop = edges.filter((e) => ids.includes(e.id));
              void onEdgesDelete(drop);
              setEdgeMenu(null);
            }}
            onClose={() => setEdgeMenu(null)}
          />
        );
      })()}
      {error && <ErrorBanner error={error} onClose={() => setError(null)} />}
    </CeWiresheetContext.Provider>
  );
}

// If multiple edges are selected, act on the whole selection; otherwise act on
// just the right-clicked edge.
function selectedEdgeIds(edges: RfEdge[], rightClickedId: string): string[] {
  const sel = edges.filter((e) => e.selected).map((e) => e.id);
  return sel.length > 1 && sel.includes(rightClickedId) ? sel : [rightClickedId];
}

function EdgeContextMenu({
  x,
  y,
  isLoopBack,
  onPrimary,
  onDelete,
  onClose,
}: {
  x: number;
  y: number;
  isLoopBack: boolean;
  onPrimary: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-edge-menu]")) return;
      onClose();
    };
    // Capture phase + pointerdown: React Flow's pane (d3-zoom) calls
    // stopImmediatePropagation on pointer/mouse down, so a bubble-phase
    // document listener never sees outside clicks. Capture fires first.
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  // Loopback edges break feedback cycles for the engine — they don't auto-fire
  // on source change, so the only way to push a value through them is a manual
  // reevaluate. Non-loopback edges auto-flow, so the useful action there is
  // promoting them to loopback (one-way: the engine never clears the flag).
  const primaryLabel = isLoopBack ? "Reevaluate" : "Set as loopback";
  return (
    <div
      data-ce-edge-menu
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
        minWidth: 160,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
      }}
    >
      <EdgeMenuItem label={primaryLabel} onClick={onPrimary} />
      <EdgeMenuItem label="Delete" onClick={onDelete} danger />
    </div>
  );
}

function EdgeMenuItem({
  label,
  onClick,
  danger,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
}) {
  const [hover, setHover] = useState(false);
  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        width: "100%",
        textAlign: "left",
        background: hover ? (danger ? "#3a1a1a" : "#232733") : "transparent",
        color: danger ? "#ffb8b8" : "#e6e8eb",
        border: "none",
        padding: "6px 10px",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: 12,
        borderRadius: 3,
      }}
    >
      {label}
    </button>
  );
}

// Parse the Details panel's alias text field ("0=off, 1=auto, 2=manual") into
// {code,label} entries; ignores blanks / malformed parts.
function parseAliasInput(s: string): Alias[] {
  const out: Alias[] = [];
  for (const part of s.split(",")) {
    const t = part.trim();
    if (!t) continue;
    const j = t.indexOf("=");
    if (j < 0) continue;
    const code = Number(t.slice(0, j).trim());
    const label = t.slice(j + 1).trim();
    if (Number.isFinite(code) && label) out.push({ code, label });
  }
  return out;
}

const detailsField: CSSProperties = {
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #2c313c",
  borderRadius: 2,
  padding: "2px 5px",
  fontSize: 11,
  fontFamily: "ui-monospace, SFMono-Regular, monospace",
  boxSizing: "border-box",
  outline: "none",
  minWidth: 0,
};

// Details panel — author the per-prop __facet (labels, units, decimals,
// aliases, hidden). Read-modify-write: starts from the current facet and
// preserves fields it doesn't edit (action/min/max/order), serialises, and
// hands the string to the caller to PATCH + reload.
function DetailsPanel({
  componentUid,
  onSave,
  onClose,
}: {
  componentUid: number;
  onSave: (facetString: string) => void;
  onClose: () => void;
}) {
  const comp = useStructural((s) => s.components.get(componentUid));
  const props = useMemo(() => {
    if (!comp) return [] as { uid: number; name: string }[];
    return Object.entries(comp.properties)
      .filter(([, p]) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL)
      .map(([name, p]) => ({ uid: p.uid, name }));
  }, [comp]);
  const initial = useMemo(
    () => facetFor(componentUid, rawFacet(comp?.properties)),
    [comp, componentUid],
  );
  type Draft = {
    label: string;
    unit: string;
    decimals: string;
    hidden: boolean;
    aliases: string;
  };
  const [draft, setDraft] = useState<Record<number, Draft>>(() => {
    const d: Record<number, Draft> = {};
    for (const p of props) {
      const f = initial.get(p.uid);
      d[p.uid] = {
        label: f?.label ?? "",
        unit: f?.unit ?? "",
        decimals: f?.decimals != null ? String(f.decimals) : "",
        hidden: f?.hidden ?? false,
        aliases: f?.aliases?.map((a) => `${a.code}=${a.label}`).join(", ") ?? "",
      };
    }
    return d;
  });
  const empty: Draft = { label: "", unit: "", decimals: "", hidden: false, aliases: "" };
  const set = (uid: number, patch: Partial<Draft>) =>
    setDraft((d) => ({ ...d, [uid]: { ...(d[uid] ?? empty), ...patch } }));

  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onEsc);
    return () => document.removeEventListener("keydown", onEsc);
  }, [onClose]);

  const save = () => {
    const facet: ComponentFacet = new Map();
    for (const p of props) {
      const d = draft[p.uid] ?? empty;
      const f: PropFacet = {};
      if (d.label.trim()) f.label = d.label.trim();
      if (d.unit.trim()) f.unit = d.unit.trim();
      const dec = Number(d.decimals);
      if (d.decimals.trim() !== "" && Number.isFinite(dec)) f.decimals = dec;
      if (d.hidden) f.hidden = true;
      const aliases = parseAliasInput(d.aliases);
      if (aliases.length) f.aliases = aliases;
      // Preserve fields this panel doesn't edit (engine / action set).
      const init = initial.get(p.uid);
      if (init?.action) f.action = init.action;
      if (init?.min != null) f.min = init.min;
      if (init?.max != null) f.max = init.max;
      if (init?.order != null) f.order = init.order;
      if (Object.keys(f).length > 0) facet.set(p.uid, f);
    }
    onSave(serializeFacet(facet));
    onClose();
  };

  return (
    <div
      onClick={onClose}
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 200,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 480,
          maxHeight: "80vh",
          background: "#1a1d24",
          border: "1px solid #2c313c",
          borderRadius: 6,
          boxShadow: "0 8px 28px rgba(0,0,0,0.6)",
          display: "flex",
          flexDirection: "column",
          color: "#e6e8eb",
          fontFamily: "-apple-system, system-ui, sans-serif",
          fontSize: 12,
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: "1px solid #2c313c",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span style={{ fontWeight: 600 }}>
            Details — <span style={{ color: "#9ecbff" }}>{comp?.name ?? componentUid}</span>
          </span>
          <span style={{ color: "#5a6172", fontSize: 10 }}>label · unit · decimals · aliases</span>
        </div>
        <div style={{ overflowY: "auto" }}>
          {props.length === 0 ? (
            <div style={{ padding: "12px", color: "#5a6172" }}>no editable properties</div>
          ) : (
            props.map((p) => {
              const d = draft[p.uid] ?? empty;
              return (
                <div key={p.uid} style={{ borderBottom: "1px solid #232733", padding: "8px 12px" }}>
                  <div
                    style={{
                      color: "#9ecbff",
                      marginBottom: 5,
                      fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    }}
                  >
                    {p.name}
                  </div>
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 64px 46px auto",
                      gap: 6,
                      alignItems: "center",
                    }}
                  >
                    <input
                      placeholder="label"
                      value={d.label}
                      onChange={(e) => set(p.uid, { label: e.target.value })}
                      onKeyDown={(e) => e.stopPropagation()}
                      style={detailsField}
                    />
                    <input
                      placeholder="unit"
                      value={d.unit}
                      onChange={(e) => set(p.uid, { unit: e.target.value })}
                      onKeyDown={(e) => e.stopPropagation()}
                      style={detailsField}
                    />
                    <input
                      placeholder="dec"
                      value={d.decimals}
                      onChange={(e) => set(p.uid, { decimals: e.target.value })}
                      onKeyDown={(e) => e.stopPropagation()}
                      style={detailsField}
                    />
                    <label
                      style={{ display: "flex", alignItems: "center", gap: 4, color: "#8892a0" }}
                    >
                      <input
                        type="checkbox"
                        checked={d.hidden}
                        onChange={(e) => set(p.uid, { hidden: e.target.checked })}
                      />
                      hide
                    </label>
                  </div>
                  <input
                    placeholder="aliases   e.g.  0=off, 1=auto, 2=manual"
                    value={d.aliases}
                    onChange={(e) => set(p.uid, { aliases: e.target.value })}
                    onKeyDown={(e) => e.stopPropagation()}
                    style={{ ...detailsField, width: "100%", marginTop: 6 }}
                  />
                </div>
              );
            })
          )}
        </div>
        <div
          style={{
            padding: "8px 12px",
            borderTop: "1px solid #2c313c",
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
          }}
        >
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              color: "#9aa3b2",
              border: "1px solid #2c313c",
              borderRadius: 3,
              padding: "4px 12px",
              cursor: "pointer",
              fontSize: 12,
            }}
          >
            Cancel
          </button>
          <button
            onClick={save}
            style={{
              background: "#2c3a55",
              color: "#9ecbff",
              border: "1px solid #3b5388",
              borderRadius: 3,
              padding: "4px 14px",
              cursor: "pointer",
              fontSize: 12,
            }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

// Node-body right-click menu. Currently "Rename / Details / Move into / Action".
// Empty-pane context menu: navigate up a folder, add a component (filterable
// picker → drops at the right-click position), and paste.
function PaneContextMenu({
  x,
  y,
  canGoUp,
  parentName,
  palette,
  canPaste,
  onUp,
  onAdd,
  onPaste,
  onClose,
}: {
  x: number;
  y: number;
  canGoUp: boolean;
  parentName: string;
  palette: PaletteExtension[];
  canPaste: boolean;
  onUp: () => void;
  onAdd: (type: string) => void;
  onPaste: () => void;
  onClose: () => void;
}) {
  const [adding, setAdding] = useState(false);
  const [filter, setFilter] = useState("");
  const [highlight, setHighlight] = useState(0);
  const hlRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    setHighlight(0);
  }, [filter, adding]);
  useEffect(() => {
    hlRef.current?.scrollIntoView({ block: "nearest" });
  }, [highlight]);

  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-node-menu]")) return;
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

  const W = adding ? 240 : 180;
  const left = Math.min(x, window.innerWidth - W - 8);
  const top = Math.min(y, window.innerHeight - (adding ? 320 : 140));

  const all = palette.flatMap((g) =>
    g.components.map((c) => ({ name: c.name, type: c.type, group: g.id })),
  );
  const f = filter.trim().toLowerCase();
  const filtered = f
    ? all.filter((c) => c.name.toLowerCase().includes(f) || c.type.toLowerCase().includes(f))
    : all;

  return (
    <div
      data-ce-node-menu
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        left,
        top,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        width: W,
        maxHeight: adding ? 320 : undefined,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      {adding ? (
        <>
          <div
            style={{
              padding: "6px 8px",
              borderBottom: "1px solid #2c313c",
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <button
              onClick={() => setAdding(false)}
              title="Back"
              style={{
                background: "transparent",
                border: "none",
                color: "#9ecbff",
                cursor: "pointer",
                fontSize: 14,
                padding: 0,
              }}
            >
              ‹
            </button>
            <input
              autoFocus
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  onClose();
                  return;
                }
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setHighlight((h) => Math.min(h + 1, Math.max(0, filtered.length - 1)));
                  return;
                }
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setHighlight((h) => Math.max(0, h - 1));
                  return;
                }
                if (e.key === "Enter") {
                  e.preventDefault();
                  const c = filtered[highlight];
                  if (c) {
                    onAdd(c.type);
                    onClose();
                  }
                  return;
                }
                e.stopPropagation();
              }}
              placeholder="Filter components…"
              style={{ ...acInput, flex: 1 }}
            />
          </div>
          <div style={{ overflowY: "auto", padding: 4 }}>
            {filtered.length === 0 ? (
              <div style={{ color: "#5a6172", padding: "6px 8px" }}>no matches</div>
            ) : (
              filtered.map((c, i) => (
                <button
                  key={c.type}
                  ref={i === highlight ? hlRef : undefined}
                  onMouseEnter={() => setHighlight(i)}
                  onClick={() => {
                    onAdd(c.type);
                    onClose();
                  }}
                  style={{ ...acBtn, background: i === highlight ? "#2c3a55" : "transparent" }}
                >
                  <span>{c.name}</span>
                  <span style={{ color: "#5a6172", fontSize: 10 }}>{c.group}</span>
                </button>
              ))
            )}
          </div>
        </>
      ) : (
        <div style={{ padding: 4 }}>
          {canGoUp && (
            <EdgeMenuItem
              label={`‹ Up to ${parentName}`}
              onClick={() => {
                onUp();
                onClose();
              }}
            />
          )}
          <EdgeMenuItem label="Add component…" onClick={() => setAdding(true)} />
          {canPaste && (
            <EdgeMenuItem
              label="Paste"
              onClick={() => {
                onPaste();
                onClose();
              }}
            />
          )}
        </div>
      )}
    </div>
  );
}

function NodeContextMenu({
  x,
  y,
  hasActions,
  canRename,
  name,
  uid,
  count,
  onRename,
  onDetails,
  onMoveInto,
  onAction,
  onClose,
}: {
  x: number;
  y: number;
  hasActions: boolean;
  canRename: boolean;
  name?: string;
  uid?: number;
  count: number;
  onRename: () => void;
  onDetails: () => void;
  onMoveInto: () => void;
  onAction: () => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    // Capture phase + pointerdown: React Flow's pane (d3-zoom) calls
    // stopImmediatePropagation on pointer/mouse down, so a bubble-phase
    // document listener never sees outside clicks. Capture fires first.
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  return (
    <div
      data-ce-node-menu
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
        minWidth: 160,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
      }}
    >
      <div
        style={{ padding: "4px 8px", color: "#8892a0", borderBottom: "1px solid #2c313c", marginBottom: 4 }}
      >
        {uid != null ? name || "component" : `${count} components`}
        <div
          style={{
            fontSize: 9,
            color: "#5a6172",
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            marginTop: 2,
          }}
        >
          {uid != null ? <CopyUid label="comp" value={uid} /> : `${count} selected`}
        </div>
      </div>
      {canRename && <EdgeMenuItem label="Rename…" onClick={onRename} />}
      {canRename && <EdgeMenuItem label="Details…" onClick={onDetails} />}
      <EdgeMenuItem label="Move into…" onClick={onMoveInto} />
      {hasActions && <EdgeMenuItem label="Action…" onClick={onAction} />}
    </div>
  );
}

// Reparent picker: choose a destination component. Lists every component in
// the current view that isn't itself being moved (plus an explicit option to
// go up to the current parent). Multi-select aware via `movingUids` — those
// components are excluded from the candidates so a node can't be reparented
// into itself, and the action runs once per moving uid.
function MoveIntoPicker({
  x,
  y,
  movingUids,
  onMove,
  onClose,
}: {
  x: number;
  y: number;
  movingUids: number[];
  onMove: (newParentUid: number) => void | Promise<void>;
  onClose: () => void;
}) {
  const [filter, setFilter] = useState("");
  // useStructural only holds the current view's children, so up-the-tree
  // moves (back to root, into an ancestor folder) wouldn't be reachable
  // without a full-tree fetch. Same pattern as ConnectPicker.
  const [allComponents, setAllComponents] = useState<Component[] | null>(null);
  const movingSet = new Set(movingUids);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        const flat: Component[] = [];
        const walk = (c: Component) => {
          flat.push(c);
          c.children?.forEach(walk);
        };
        // Include root itself in the candidate list — it's a legitimate
        // destination (move out of any folder back to the top level).
        resp.nodes.forEach(walk);
        setAllComponents(flat);
      } catch {
        if (cancelled) return;
        setAllComponents([...useStructural.getState().components.values()]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // A component can't be reparented into itself or into one of its own
  // descendants (cycle). Path-based detection: a descendant's path starts
  // with the moving component's path + "/".
  const movingPaths: string[] = (allComponents ?? [])
    .filter((c) => movingSet.has(c.uid))
    .map((c) => c.path);
  const isMovingOrDescendant = (path: string): boolean => {
    for (const mp of movingPaths) {
      if (path === mp || path.startsWith(mp + "/")) return true;
    }
    return false;
  };

  interface Candidate {
    uid: number;
    name: string;
    kind: string;
    path: string;
    tier: number;
  }
  // Order destinations by relationship to the folder the moving component is in:
  // up one level (its folder's parent) → same level (its siblings) → children
  // (deeper inside the current folder) → everything else.
  const movingComp = (allComponents ?? []).find((c) => movingSet.has(c.uid));
  const curFolderUid = movingComp?.parent; // the folder we're moving FROM
  const curFolder = (allComponents ?? []).find((c) => c.uid === curFolderUid);
  const upUid = curFolder?.parent; // one level up
  const curFolderPath = curFolder?.path;
  const tierOf = (c: Component): number => {
    if (upUid !== undefined && c.uid === upUid) return 0; // up one level
    if (curFolderUid !== undefined && c.parent === curFolderUid) return 1; // same level
    if (curFolderPath && c.path.startsWith(curFolderPath + "/")) return 2; // children
    return 3; // everything else
  };
  const candidates: Candidate[] = [];
  for (const c of allComponents ?? []) {
    if (movingSet.has(c.uid)) continue;
    if (isMovingOrDescendant(c.path)) continue;
    candidates.push({
      uid: c.uid,
      name: c.name || c.type,
      kind: c.type,
      path: c.path,
      tier: tierOf(c),
    });
  }
  // Tier first (preference order), then path so each tier stays clustered.
  candidates.sort((a, b) => (a.tier !== b.tier ? a.tier - b.tier : a.path.localeCompare(b.path)));

  const f = filter.trim().toLowerCase();
  const visible = f
    ? candidates.filter(
        (c) =>
          c.name.toLowerCase().includes(f) ||
          c.kind.toLowerCase().includes(f) ||
          c.path.toLowerCase().includes(f),
      )
    : candidates;

  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    // Capture phase + pointerdown: React Flow's pane (d3-zoom) calls
    // stopImmediatePropagation on pointer/mouse down, so a bubble-phase
    // document listener never sees outside clicks. Capture fires first.
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);

  const PICKER_W = 260;
  const left = Math.min(x, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 320);
  return (
    <div
      data-ce-node-menu
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
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #2c313c" }}>
        <div style={{ color: "#8892a0", fontSize: 10, marginBottom: 4 }}>
          Move {movingUids.length === 1 ? "1 component" : `${movingUids.length} components`} into…
        </div>
        <input
          autoFocus
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
            else if (e.key === "Enter" && visible.length === 1) onMove(visible[0].uid);
            e.stopPropagation();
          }}
          placeholder="filter…"
          style={{
            width: "100%",
            background: "#0f1115",
            color: "#e6e8eb",
            border: "1px solid #2c313c",
            borderRadius: 2,
            padding: "3px 6px",
            fontSize: 12,
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            boxSizing: "border-box",
            outline: "none",
          }}
        />
      </div>
      <div style={{ flex: 1, overflowY: "auto" }}>
        {visible.length === 0 ? (
          <div style={{ padding: "10px 8px", color: "#5a6172", fontSize: 12 }}>
            {allComponents == null ? "loading…" : "no destinations"}
          </div>
        ) : (
          visible.map((c, idx) => {
            // Drop the leading "root/" from the displayed path so the column
            // reads cleanly; bare "root" shows as the explicit top-level
            // option.
            const pathLabel =
              c.path === "root" ? "root" : c.path.startsWith("root/") ? c.path.slice(5) : c.path;
            const showSection = c.tier !== (idx > 0 ? visible[idx - 1].tier : -1);
            const sectionLabel =
              c.tier === 0
                ? "up one level"
                : c.tier === 1
                  ? "same level"
                  : c.tier === 2
                    ? "inside this folder"
                    : "other";
            return (
              <div key={c.uid}>
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
                  onClick={() => onMove(c.uid)}
                style={{
                  display: "flex",
                  width: "100%",
                  textAlign: "left",
                  padding: "5px 8px",
                  background: "transparent",
                  color: "#e6e8eb",
                  border: "none",
                  cursor: "pointer",
                  fontSize: 12,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  alignItems: "baseline",
                  gap: 6,
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#2c313c")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                <span
                  style={{
                    color: "#9ecbff",
                    flex: 1,
                    minWidth: 0,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                  title={c.path}
                >
                  {pathLabel}
                </span>
                <span style={{ color: "#5a6172", fontSize: 11, flexShrink: 0 }}>{c.kind}</span>
                </button>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

// Node-body "Action…" popup. Same chrome as MoveIntoPicker, positioned at the
// right-click point. Acts on the current selection (`targetUids`). Body is a
// placeholder for now — the actual actions get listed here once defined.
// ---- Action picker ---------------------------------------------------------
// Opens from the right-click "Action…" item. Lists the actions available on the
// selected component(s) (resolved by the caller via `getActionsFor`), builds a
// params form from each action's signature, invokes `POST /call/nodes/uid/{uid}`
// per target, and shows the `returns`.

const acInput: CSSProperties = {
  background: "#0f1115",
  border: "1px solid #2c313c",
  borderRadius: 4,
  color: "#e6e8eb",
  fontSize: 12,
  padding: "4px 6px",
  fontFamily: "inherit",
};
const acBtn: CSSProperties = {
  width: "100%",
  textAlign: "left",
  background: "transparent",
  color: "#e6e8eb",
  border: "none",
  borderRadius: 4,
  padding: "6px 8px",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: 12,
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: 6,
};
const acBtnPrimary: CSSProperties = {
  width: "100%",
  background: "#2d6cdf",
  color: "#fff",
  border: "none",
  borderRadius: 4,
  padding: "7px 8px",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: 12,
  marginTop: 6,
};
const acRow: CSSProperties = { display: "flex", justifyContent: "space-between", padding: "2px 0" };

// Map a FlexValue type tag onto an input kind. Numeric tags cover the engine's
// int/float family; everything non-bool/non-numeric renders as text.
function actionKind(type: string): "bool" | "num" | "str" {
  const t = type.toLowerCase();
  if (t === "bool" || t === "boolean") return "bool";
  if (/^(u?int\d*|[iuf]\d+|float|double|number)$/.test(t)) return "num";
  return "str";
}
function defaultForType(type: string): FlexValue {
  const k = actionKind(type);
  return k === "bool" ? false : k === "num" ? 0 : "";
}
function coerceParam(type: string, raw: string): FlexValue {
  const k = actionKind(type);
  if (k === "num") {
    const n = Number(raw);
    return Number.isFinite(n) ? n : 0;
  }
  if (k === "bool") return raw === "true" || raw === "1";
  return raw;
}

function ParamField({
  def,
  value,
  onChange,
}: {
  def: ActionParamDef;
  value: FlexValue;
  onChange: (v: FlexValue) => void;
}) {
  const kind = actionKind(def.type);
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 3, margin: "0 0 8px" }}>
      <span style={{ color: "#8892a0", fontSize: 10 }}>
        {def.label ?? def.name}
        <span style={{ color: "#5a6172" }}> · {def.type}</span>
      </span>
      {def.enum ? (
        <select
          value={String(value ?? "")}
          onChange={(e) => onChange(coerceParam(def.type, e.target.value))}
          style={acInput}
        >
          {def.enum.map((opt) => (
            <option key={String(opt)} value={String(opt)}>
              {String(opt)}
            </option>
          ))}
        </select>
      ) : kind === "bool" ? (
        <input
          type="checkbox"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked)}
          style={{ width: 14, height: 14 }}
        />
      ) : (
        <input
          type={kind === "num" ? "number" : "text"}
          value={value === null || value === undefined ? "" : String(value)}
          onChange={(e) => onChange(coerceParam(def.type, e.target.value))}
          style={acInput}
        />
      )}
    </label>
  );
}

function ActionPicker({
  x,
  y,
  actions,
  targetUids,
  onInvoke,
  onClose,
}: {
  x: number;
  y: number;
  actions: ActionDef[];
  targetUids: number[];
  onInvoke: (
    uids: number[],
    action: string,
    params: Record<string, FlexValue>,
  ) => Promise<Array<{ returns: Record<string, FlexValue> }>>;
  onClose: () => void;
}) {
  const [selected, setSelected] = useState<ActionDef | null>(null);
  const [values, setValues] = useState<Record<string, FlexValue>>({});
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<Record<string, FlexValue> | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const dismiss = (e: MouseEvent) => {
      const el = e.target as Element | null;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // Capture phase + pointerdown: React Flow's pane (d3-zoom) calls
    // stopImmediatePropagation on pointer/mouse down, so a bubble-phase
    // document listener never sees outside clicks. Capture fires first.
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
      document.removeEventListener("keydown", onEsc);
    };
  }, [onClose]);

  const run = async (a: ActionDef, params: Record<string, FlexValue>) => {
    setBusy(true);
    setError(null);
    try {
      const res = await onInvoke(targetUids, a.name, params);
      setResult(res[0]?.returns ?? {});
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  // Click an action: invoke immediately if it has no params, else open its form
  // seeded with each param's default.
  const choose = (a: ActionDef) => {
    setError(null);
    setResult(null);
    if (!a.params || a.params.length === 0) {
      void run(a, {});
      return;
    }
    const init: Record<string, FlexValue> = {};
    for (const p of a.params) init[p.name] = p.default ?? defaultForType(p.type);
    setValues(init);
    setSelected(a);
  };

  const PICKER_W = 280;
  const left = Math.min(x, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 360);
  const count = targetUids.length;

  return (
    <div
      data-ce-node-menu
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
        maxHeight: 360,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "6px 8px",
          borderBottom: "1px solid #2c313c",
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        {selected && !result && (
          <button
            onClick={() => {
              setSelected(null);
              setError(null);
            }}
            title="Back"
            style={{
              background: "transparent",
              border: "none",
              color: "#9ecbff",
              cursor: "pointer",
              fontSize: 14,
              padding: 0,
            }}
          >
            ‹
          </button>
        )}
        <div style={{ color: "#8892a0", fontSize: 10, flex: 1 }}>
          {result
            ? "Result"
            : selected
              ? selected.label ?? selected.name
              : `Action on ${count === 1 ? "1 component" : `${count} components`}`}
        </div>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 8 }}>
        {result ? (
          <div>
            {Object.keys(result).length === 0 ? (
              <div style={{ color: "#5a6172" }}>done — no return values</div>
            ) : (
              Object.entries(result).map(([k, v]) => (
                <div key={k} style={acRow}>
                  <span style={{ color: "#8892a0" }}>{k}</span>
                  <span style={{ color: "#e6e8eb", fontVariantNumeric: "tabular-nums" }}>
                    {String(v)}
                  </span>
                </div>
              ))
            )}
            <button onClick={onClose} style={acBtnPrimary}>
              Close
            </button>
          </div>
        ) : selected ? (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(selected, values);
            }}
          >
            {(selected.params ?? []).map((p) => (
              <ParamField
                key={p.name}
                def={p}
                value={values[p.name]}
                onChange={(v) => setValues((cur) => ({ ...cur, [p.name]: v }))}
              />
            ))}
            {error && <div style={{ color: "#ffb8b8", margin: "6px 0" }}>{error}</div>}
            <button type="submit" disabled={busy} style={acBtnPrimary}>
              {busy ? "Running…" : `Run on ${count === 1 ? "1 component" : `${count} components`}`}
            </button>
          </form>
        ) : actions.length === 0 ? (
          <div style={{ color: "#5a6172" }}>no actions for this component</div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {error && <div style={{ color: "#ffb8b8", margin: "2px 0 6px" }}>{error}</div>}
            {actions.map((a) => (
              <button
                key={a.name}
                onClick={() => choose(a)}
                disabled={busy}
                title={a.description}
                style={acBtn}
              >
                <span>{a.label ?? a.name}</span>
                {a.params && a.params.length > 0 ? (
                  <span style={{ color: "#5a6172", fontSize: 10 }}>…</span>
                ) : null}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function sanitizeName(type: string): string {
  // type looks like "vendor-ext::ComponentName". Use the local segment, strip anything
  // that isn't alphanumeric or underscore so the engine's name validator accepts it.
  const idx = type.lastIndexOf("::");
  const local = idx >= 0 ? type.slice(idx + 2) : type;
  const cleaned = local.replace(/[^A-Za-z0-9_]/g, "");
  return cleaned || "node";
}

function buildRfNodes(
  comps: Component[],
  onEnter: (uid: number) => void,
  onContextMenu: (uid: number, x: number, y: number) => void,
  // Carry the existing selection set across reloads — without this, any topology
  // event that fires while a node is selected wipes the selection on the next
  // setNodes call.
  selectedIds?: Set<string>,
  // Component types that declare actions (from /schema) — drives the ⚡ marker.
  actionTypes?: Set<string>,
): RfNode<FunctionBlockData>[] {
  // If every node has position (0,0) — i.e. the engine hasn't laid them out yet — fall
  // back to a grid. As soon as the user drags a node, that node's position is persisted
  // (PATCH /nodes/uid/{uid}), so subsequent loads use the saved layout.
  const allZero = comps.every(
    (c) => (c.metadata?.position?.x ?? 0) === 0 && (c.metadata?.position?.y ?? 0) === 0,
  );
  const cols = Math.max(1, Math.ceil(Math.sqrt(comps.length)));
  const GRID_X = NODE_W + 60;
  const GRID_Y = 220;
  return comps.map((c, i) => {
    const px = c.metadata?.position?.x ?? 0;
    const py = c.metadata?.position?.y ?? 0;
    const pos = allZero
      ? { x: (i % cols) * GRID_X, y: Math.floor(i / cols) * GRID_Y }
      : { x: px, y: py };
    const id = String(c.uid);
    return {
      id,
      type: "fb",
      position: pos,
      width: NODE_W,
      data: {
        componentUid: c.uid,
        name: c.name,
        hasChildren: (c.childrenCount ?? 0) > 0,
        childCount: c.childrenCount ?? 0,
        hasActions: actionTypes?.has(c.type) ?? false,
        onEnter,
        onContextMenu,
      },
      draggable: true,
      selected: selectedIds?.has(id) ?? false,
    };
  });
}

// MiniMap dot colors — keyed off the same node shape buildRfNodes emits.
// Selected nodes pop in the accent so you can spot your selection on the map.
function miniMapNodeColor(n: RfNode): string {
  if (n.selected) return "#6cb1ff";
  if (n.type === "ghost") return "#5a626e";
  return (n.data as { hasChildren?: boolean })?.hasChildren ? "#4f80c4" : "#7b8593";
}
function miniMapNodeStroke(n: RfNode): string {
  if (n.selected) return "#8cc4ff";
  return (n.data as { hasChildren?: boolean })?.hasChildren ? "#6cb1ff" : "#9aa3b2";
}

function buildRfEdges(edges: Edge[], comps: Component[]): RfEdge[] {
  const cByUid = new Map<number, Component>();
  for (const c of comps) cByUid.set(c.uid, c);
  const out: RfEdge[] = [];
  for (const e of edges) {
    const src = cByUid.get(e.sourceUid);
    const dst = cByUid.get(e.targetUid);
    const srcProp = src?.properties[e.sourceProperty];
    const dstProp = dst?.properties[e.targetProperty];
    if (!srcProp || !dstProp) continue;
    // loopBack edges close a feedback cycle and the engine treats them as a one-cycle
    // delay boundary — render them dotted in a muted grey so they read as "logically
    // present but not in the direct dataflow".
    const isLoop = e.loopBack === true;
    out.push({
      id: String(e.uid),
      source: String(e.sourceUid),
      sourceHandle: String(srcProp.uid),
      target: String(e.targetUid),
      targetHandle: String(dstProp.uid),
      style: isLoop
        ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" }
        : { stroke: "#4a9eff", strokeWidth: 1.5 },
      animated: false,
    });
  }
  return out;
}

function Breadcrumb({ crumbs, onGoTo }: { crumbs: Crumb[]; onGoTo: (i: number) => void }) {
  return (
    <div
      style={{
        position: "fixed",
        bottom: 12,
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 20,
        display: "flex",
        alignItems: "center",
        background: "rgba(20,23,30,0.92)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        padding: "6px 12px",
        gap: 6,
        fontSize: 12,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        maxWidth: "70%",
        overflowX: "auto",
      }}
    >
      {crumbs.map((c, i) => {
        const last = i === crumbs.length - 1;
        return (
          <span key={c.uid} style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <button
              onClick={() => onGoTo(i)}
              disabled={last}
              style={{
                background: "transparent",
                color: last ? "#e6e8eb" : "#9ecbff",
                border: "none",
                padding: 0,
                cursor: last ? "default" : "pointer",
                fontFamily: "inherit",
                fontSize: 12,
                fontWeight: last ? 600 : 400,
              }}
            >
              {c.name}
            </button>
            {!last && <span style={{ color: "#5a6172" }}>/</span>}
          </span>
        );
      })}
    </div>
  );
}

// Left-side palette: collapsible list of extensions, each expanding to its components.
// Click an extension row to toggle. Components are draggable onto the canvas (drop is
// handled by the ReactFlow wrapper) AND double-clickable for one-click add.
function Palette({
  palette,
  onAdd,
  currentParentUid,
}: {
  palette: PaletteExtension[];
  onAdd: (type: string) => void;
  currentParentUid: number;
}) {
  // Collapsed state persists in localStorage so the canvas stays clean on reload if the
  // user explicitly hid the palette before.
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    try {
      return window.localStorage.getItem("ce-ui.palette.collapsed") === "1";
    } catch {
      return false;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.palette.collapsed", collapsed ? "1" : "0");
    } catch {
      /* private mode etc */
    }
  }, [collapsed]);

  const [open, setOpen] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");
  const toggle = (id: string) =>
    setOpen((cur) => {
      const next = new Set(cur);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const f = filter.trim().toLowerCase();
  const filtered = useMemo(() => {
    if (!f) return palette;
    return palette
      .map((g) => ({
        ...g,
        components: g.components.filter(
          (c) => c.name.toLowerCase().includes(f) || c.type.toLowerCase().includes(f),
        ),
      }))
      .filter((g) => g.components.length > 0);
  }, [palette, f]);

  // While filtering, auto-expand any group that has matches so the user sees results.
  const effectivelyOpen = (id: string) => (f ? true : open.has(id));

  // Collapsed: render only a slim "show" tab on the left edge so the canvas reclaims
  // the space. Click anywhere on the tab to expand.
  if (collapsed) {
    return (
      <button
        onClick={() => setCollapsed(false)}
        title="Show component palette"
        style={{
          position: "fixed",
          top: 12,
          left: 12,
          zIndex: 20,
          width: 28,
          height: 80,
          background: "rgba(20,23,30,0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          color: "#cbd3e0",
          cursor: "pointer",
          fontFamily: "inherit",
          fontSize: 14,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        ▸
      </button>
    );
  }

  return (
    <div
      style={{
        position: "fixed",
        top: 12,
        left: 12,
        bottom: 12,
        zIndex: 20,
        background: "rgba(20, 23, 30, 0.96)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        color: "#e6e8eb",
        fontSize: 12,
        width: 260,
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ padding: "10px 12px", borderBottom: "1px solid #2c313c" }}>
        <div style={{ display: "flex", alignItems: "center", marginBottom: 8 }}>
          <div style={{ fontWeight: 600, fontSize: 13, flex: 1 }}>Add component</div>
          <button
            onClick={() => setCollapsed(true)}
            title="Hide palette"
            style={{
              background: "transparent",
              border: "none",
              color: "#8892a0",
              cursor: "pointer",
              fontFamily: "inherit",
              fontSize: 14,
              padding: "0 4px",
              lineHeight: 1,
            }}
          >
            ◂
          </button>
        </div>
        <input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="filter…"
          spellCheck={false}
          style={{
            width: "100%",
            background: "#222731",
            color: "#cbd3e0",
            border: "1px solid #2c313c",
            borderRadius: 3,
            padding: "4px 6px",
            fontSize: 11,
            fontFamily: "ui-monospace, monospace",
            boxSizing: "border-box",
          }}
        />
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "4px 0" }}>
        {filtered.length === 0 && (
          <div style={{ padding: "10px 12px", color: "#5a6172", fontSize: 11 }}>no matches</div>
        )}
        {filtered.map((g) => {
          const isOpen = effectivelyOpen(g.id);
          return (
            <div key={g.id}>
              <button
                onClick={() => toggle(g.id)}
                style={{
                  width: "100%",
                  textAlign: "left",
                  background: "transparent",
                  color: "#e6e8eb",
                  border: "none",
                  padding: "6px 12px",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  fontSize: 12,
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <span style={{ color: "#8892a0", fontFamily: "ui-monospace, monospace", width: 8 }}>
                  {isOpen ? "▾" : "▸"}
                </span>
                <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {g.id}
                </span>
                <span style={{ color: "#5a6172", fontSize: 10 }}>{g.components.length}</span>
              </button>
              {isOpen && (
                <div style={{ paddingBottom: 4 }}>
                  {g.components.map((c) => (
                    <PaletteItem key={c.type} component={c} onAdd={() => onAdd(c.type)} />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
      <div
        style={{
          padding: "8px 12px",
          borderTop: "1px solid #2c313c",
          fontSize: 10,
          color: "#8892a0",
          lineHeight: 1.5,
        }}
      >
        drag onto canvas, or double-click to add at center.<br />
        drag handle → handle to connect • Delete to remove.
        <div style={{ marginTop: 4, color: "#5a6172" }}>parent uid: {currentParentUid}</div>
      </div>
    </div>
  );
}

function PaletteItem({
  component,
  onAdd,
}: {
  component: PaletteComponent;
  onAdd: () => void;
}) {
  const [dragging, setDragging] = useState(false);
  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.effectAllowed = "copy";
        e.dataTransfer.setData(DND_TYPE, component.type);
        setDragging(true);
      }}
      onDragEnd={() => setDragging(false)}
      onDoubleClick={onAdd}
      title={`${component.type} — double-click to add, drag to drop on canvas`}
      style={{
        margin: "0 8px 2px 8px",
        padding: "4px 8px 4px 22px",
        background: dragging ? "#2c3a55" : "#1a1d24",
        color: "#cbd3e0",
        border: "1px solid #2c313c",
        borderRadius: 3,
        cursor: "grab",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        userSelect: "none",
        display: "flex",
        flexDirection: "column",
        gap: 1,
      }}
    >
      <span style={{ color: "#e6e8eb" }}>{component.name}</span>
      <span style={{ fontSize: 9, color: "#5a6172", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {component.type}
      </span>
    </div>
  );
}

function ErrorBanner({
  error,
  onClose,
}: {
  error: { message: string; debug?: string };
  onClose: () => void;
}) {
  const [copied, setCopied] = useState(false);
  const copy = (e: React.MouseEvent) => {
    e.stopPropagation();
    const text = error.debug ?? error.message;
    void navigator.clipboard?.writeText(text).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      },
      () => {},
    );
  };
  return (
    <div
      style={{
        position: "fixed",
        bottom: 12,
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 30,
        maxWidth: "min(720px, 90vw)",
        background: "#3a1a1a",
        border: "1px solid #6b2a2a",
        color: "#ffb8b8",
        padding: "6px 10px",
        borderRadius: 4,
        fontSize: 12,
        fontFamily: "ui-monospace, monospace",
        display: "flex",
        alignItems: "flex-start",
        gap: 8,
      }}
    >
      <span style={{ whiteSpace: "pre-wrap", overflow: "hidden", flex: 1, maxHeight: 120 }}>
        {error.message}
      </span>
      <button
        onClick={copy}
        title={error.debug ? "Copy request + response" : "Copy error"}
        style={{
          flexShrink: 0,
          background: "#5a2a2a",
          color: "#ffd8d8",
          border: "1px solid #6b2a2a",
          borderRadius: 3,
          padding: "1px 8px",
          fontSize: 11,
          cursor: "pointer",
          fontFamily: "inherit",
        }}
      >
        {copied ? "copied" : "copy"}
      </button>
      <button
        onClick={onClose}
        title="Dismiss"
        style={{
          flexShrink: 0,
          background: "transparent",
          color: "#ffb8b8",
          border: "none",
          fontSize: 13,
          cursor: "pointer",
          lineHeight: 1,
          padding: "0 2px",
        }}
      >
        ✕
      </button>
    </div>
  );
}
