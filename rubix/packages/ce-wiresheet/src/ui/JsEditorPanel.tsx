// `jsEditor` widget — a tabbed editor for jsLogic components (NubeIO-js).
//
// Shape (like IDE editor tabs):
//   - a pinned "All" tab listing every jsLogic globally (name · path · bound
//     scriptId); clicking one opens/focuses its tab.
//   - one closeable tab per opened jsLogic (label = component name). Its body is
//     the per-component source editor (source via jsScriptStore get/putScript,
//     live `log` stream, Save&Assign via the jsLogic's setScript).
//
// Open editors stay mounted (display-toggled) so edits + log survive tab
// switches; only the ACTIVE tab subscribes to the value stream (the host's
// prop-subscription replaces the whole set, so only one editor may own it).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import { autocompletion } from "@codemirror/autocomplete";
import { oneDark } from "@codemirror/theme-one-dark";
import { Save, RefreshCw, Trash2, Plus, X, FileCode2, Locate, BookOpen, Copy } from "lucide-react";
import { useStructural, useValues } from "../lib/store";
import type { Component, FlexValue } from "../lib/engine-types";
import { getNodeByUid, getRootNodes } from "../lib/rest";
import { resolveJsStore } from "./jsScriptStore";
import { loadJsApi, jsCompletionSource, loadExamples, type JsSymbol, type JsExample } from "./jsApi";
import { registerWidget, type WidgetProps } from "./registry";
import type { RenderCtx } from "./registry";

const MAX_LOG_LINES = 500;
const asStr = (v: unknown): string => (typeof v === "string" ? v : v == null ? "" : String(v));
const splitIds = (s: string): string[] => s.split(",").map((x) => x.trim()).filter(Boolean);

interface JsRow { uid: number; name: string; path?: string; scriptId: string }

/** Global jsLogic list with each component's bound scriptId (values:true). */
async function loadJsLogic(fullType: string, scriptIdProp: string): Promise<JsRow[]> {
  const r = await getRootNodes({ type: fullType, values: true });
  return r.nodes.map((c) => ({
    uid: c.uid,
    name: c.name ?? `#${c.uid}`,
    path: c.path,
    scriptId: asStr(c.properties?.[scriptIdProp]?.value),
  }));
}

// ---------------------------------------------------------------------------
// Container: tab bar + "All" index + one mounted editor per open component.
// ---------------------------------------------------------------------------
const EX = (label: string) => `ex:${label}`;

function JsEditorPanel({ node, ctx }: WidgetProps) {
  const fullType = (node.fullType as string) ?? "NubeIO-js::jsLogic";
  const serviceType = (node.serviceType as string) ?? "jsScriptStore";
  const scriptIdProp = (node.scriptIdProp as string) ?? "scriptId";
  const exampleAction = (node.exampleAction as string) ?? "getExamples";

  // `ctx` is rebuilt each render — read via a ref so effects key on real data.
  const ctxRef = useRef(ctx);
  ctxRef.current = ctx;

  // Resolve the jsScriptStore singleton once; shared by every editor tab.
  const [service, setService] = useState<Component | null>(null);
  const [serviceState, setServiceState] = useState<"loading" | "ok" | "missing">("loading");
  useEffect(() => {
    let alive = true;
    setServiceState("loading");
    resolveJsStore(serviceType)
      .then((svc) => { if (!alive) return; if (svc) { setService(svc); setServiceState("ok"); } else setServiceState("missing"); })
      .catch(() => alive && setServiceState("missing"));
    return () => { alive = false; };
  }, [serviceType]);
  const serviceUid = service?.uid ?? null;

  // Global jsLogic list for the "All" index, polled so it tracks adds/removes.
  const [rows, setRows] = useState<JsRow[]>([]);
  useEffect(() => {
    let alive = true;
    const tick = () => loadJsLogic(fullType, scriptIdProp).then((r) => { if (alive) setRows(r); }).catch(() => {});
    tick();
    const id = setInterval(tick, 4000);
    return () => { alive = false; clearInterval(id); };
  }, [fullType, scriptIdProp]);

  // Example library (cached) for the "Examples" tab.
  const [examples, setExamples] = useState<JsExample[]>([]);
  useEffect(() => {
    const call = ctxRef.current.callAction;
    if (serviceUid == null || !call) return;
    let alive = true;
    loadExamples(call, serviceUid, exampleAction).then((e) => { if (alive) setExamples(e); }).catch(() => {});
    return () => { alive = false; };
  }, [serviceUid, exampleAction]);

  const [openUids, setOpenUids] = useState<number[]>([]);
  const [openExamples, setOpenExamples] = useState<string[]>([]);
  const [active, setActive] = useState<number | string>("index");

  // Per-tab dirty flags (reported by each editor) for the close confirmation.
  const dirtyRef = useRef<Record<number, boolean>>({});
  const [, force] = useState(0);
  const onDirty = useCallback((uid: number, d: boolean) => {
    if (dirtyRef.current[uid] !== d) { dirtyRef.current[uid] = d; force((x) => x + 1); }
  }, []);

  const openTab = useCallback((uid: number) => {
    setOpenUids((p) => (p.includes(uid) ? p : [...p, uid]));
    setActive(uid);
  }, []);
  const closeTab = useCallback((uid: number) => {
    if (dirtyRef.current[uid] && !window.confirm("Discard unsaved changes to this script?")) return;
    delete dirtyRef.current[uid];
    setOpenUids((p) => {
      const i = p.indexOf(uid);
      const next = p.filter((u) => u !== uid);
      setActive((a) => (a !== uid ? a : next[i] ?? next[i - 1] ?? "index"));
      return next;
    });
  }, []);
  const openExample = useCallback((label: string) => {
    setOpenExamples((p) => (p.includes(label) ? p : [...p, label]));
    setActive(EX(label));
  }, []);
  const closeExample = useCallback((label: string) => {
    setOpenExamples((p) => {
      const i = p.indexOf(label);
      const next = p.filter((l) => l !== label);
      setActive((a) => (a !== EX(label) ? a : next[i] ? EX(next[i]) : next[i - 1] ? EX(next[i - 1]) : "examples"));
      return next;
    });
  }, []);

  const nameOf = (uid: number) => rows.find((r) => r.uid === uid)?.name ?? `#${uid}`;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, color: "#e6e8eb" }}>
      {/* tab bar */}
      <div style={{ display: "flex", alignItems: "stretch", gap: 1, background: "#15181e", borderBottom: "1px solid #2c313c", flexShrink: 0, overflowX: "auto" }}>
        <Tab active={active === "index"} onClick={() => setActive("index")} pinned>
          <FileCode2 size={13} /> All
        </Tab>
        <Tab active={active === "examples"} onClick={() => setActive("examples")} pinned>
          <BookOpen size={13} /> Examples
        </Tab>
        {openUids.map((uid) => (
          <Tab key={uid} active={active === uid} onClick={() => setActive(uid)}>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 160 }}>
              {nameOf(uid)}{dirtyRef.current[uid] ? " ●" : ""}
            </span>
            {ctx.locate && (
              <span role="button" title="Locate on canvas" onClick={(e) => { e.stopPropagation(); ctx.locate!(uid); }} style={iconBtn}>
                <Locate size={12} />
              </span>
            )}
            <span role="button" title="Close" onClick={(e) => { e.stopPropagation(); closeTab(uid); }} style={iconBtn}>
              <X size={12} />
            </span>
          </Tab>
        ))}
        {openExamples.map((label) => (
          <Tab key={EX(label)} active={active === EX(label)} onClick={() => setActive(EX(label))}>
            <BookOpen size={12} />
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 140 }}>{label}</span>
            <span role="button" title="Close" onClick={(e) => { e.stopPropagation(); closeExample(label); }} style={iconBtn}>
              <X size={12} />
            </span>
          </Tab>
        ))}
      </div>

      {/* body — index + examples list + mounted editors/viewers, display-toggled */}
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
        <div style={{ position: "absolute", inset: 0, display: active === "index" ? "block" : "none", overflow: "auto" }}>
          <IndexList rows={rows} openUids={openUids} active={active} onOpen={openTab} serviceState={serviceState} />
        </div>
        <div style={{ position: "absolute", inset: 0, display: active === "examples" ? "block" : "none", overflow: "auto" }}>
          <ExamplesList examples={examples} openExamples={openExamples} active={active} onOpen={openExample} serviceState={serviceState} />
        </div>
        {openUids.map((uid) => (
          <div key={uid} style={{ position: "absolute", inset: 0, display: active === uid ? "block" : "none" }}>
            <JsScriptEditor componentUid={uid} active={active === uid} node={node} ctx={ctx} service={service} serviceState={serviceState} onDirty={onDirty} />
          </div>
        ))}
        {openExamples.map((label) => {
          const ex = examples.find((e) => e.label === label);
          return (
            <div key={EX(label)} style={{ position: "absolute", inset: 0, display: active === EX(label) ? "block" : "none" }}>
              {ex ? <ExampleViewer ex={ex} /> : <div style={{ padding: 12, color: "#5a6172", fontSize: 12 }}>example not found</div>}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Tab({ active, onClick, pinned, children }: { active: boolean; onClick: () => void; pinned?: boolean; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 5,
        padding: "6px 10px",
        fontSize: 12,
        cursor: "pointer",
        border: "none",
        borderRight: "1px solid #2c313c",
        borderBottom: active ? "2px solid #3b6eff" : "2px solid transparent",
        background: active ? "#1d2128" : "transparent",
        color: active ? "#e6e8eb" : "#9aa3b2",
        whiteSpace: "nowrap",
        flexShrink: 0,
        fontWeight: pinned ? 600 : 400,
      }}
    >
      {children}
    </button>
  );
}

function IndexList({ rows, openUids, active, onOpen, serviceState }: {
  rows: JsRow[]; openUids: number[]; active: number | string; onOpen: (uid: number) => void; serviceState: string;
}) {
  return (
    <div style={{ padding: 8, display: "flex", flexDirection: "column", gap: 4 }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, padding: "2px 4px 6px" }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>jsLogic components</span>
        <span style={{ color: "#5a6172", fontSize: 11 }}>({rows.length})</span>
        {serviceState === "missing" && <span style={{ color: "#e0707a", fontSize: 11, marginLeft: 6 }}>jsScriptStore not found</span>}
      </div>
      {rows.length === 0 ? (
        <div style={{ color: "#5a6172", fontSize: 12, padding: 6 }}>No jsLogic components on this engine.</div>
      ) : (
        rows.map((r) => {
          const folder = r.path ? r.path.replace(/^root\/?/, "").split("/").slice(0, -1).join("/") : "";
          const isOpen = openUids.includes(r.uid);
          return (
            <button
              key={r.uid}
              onClick={() => onOpen(r.uid)}
              style={{
                display: "flex", alignItems: "center", gap: 8, padding: "7px 9px", textAlign: "left",
                background: active === r.uid ? "#1d2330" : "#1a1d24", border: "1px solid #2c313c", borderRadius: 5,
                cursor: "pointer", color: "#e6e8eb", fontSize: 12,
              }}
            >
              <span style={{ fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.name}</span>
              {folder && <span style={{ color: "#5a6172", fontSize: 10 }}>{folder}</span>}
              <span style={{ flex: 1 }} />
              <span style={{ color: r.scriptId ? "#9ecbff" : "#7a6a3a", fontSize: 11, fontFamily: "ui-monospace, monospace" }}>
                {r.scriptId || "no script"}
              </span>
              {isOpen && <span style={{ color: "#5a6172", fontSize: 10 }}>open</span>}
            </button>
          );
        })
      )}
    </div>
  );
}

// Examples index — same layout as the components list, listing the library from
// getExamples; clicking one opens a read-only viewer tab to copy from.
function ExamplesList({ examples, openExamples, active, onOpen, serviceState }: {
  examples: JsExample[]; openExamples: string[]; active: number | string; onOpen: (label: string) => void; serviceState: string;
}) {
  return (
    <div style={{ padding: 8, display: "flex", flexDirection: "column", gap: 4 }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, padding: "2px 4px 6px" }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>example scripts</span>
        <span style={{ color: "#5a6172", fontSize: 11 }}>({examples.length})</span>
        {serviceState === "missing" && <span style={{ color: "#e0707a", fontSize: 11, marginLeft: 6 }}>jsScriptStore not found</span>}
      </div>
      {examples.length === 0 ? (
        <div style={{ color: "#5a6172", fontSize: 12, padding: 6 }}>No examples available.</div>
      ) : (
        examples.map((ex) => (
          <button
            key={ex.label}
            onClick={() => onOpen(ex.label)}
            style={{
              display: "flex", alignItems: "center", gap: 8, padding: "7px 9px", textAlign: "left",
              background: active === EX(ex.label) ? "#1d2330" : "#1a1d24", border: "1px solid #2c313c", borderRadius: 5,
              cursor: "pointer", color: "#e6e8eb", fontSize: 12,
            }}
          >
            <BookOpen size={13} style={{ flexShrink: 0, color: "#8892a0" }} />
            <span style={{ fontWeight: 500, flexShrink: 0 }}>{ex.label}</span>
            {ex.desc && <span style={{ color: "#5a6172", fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{ex.desc}</span>}
            <span style={{ flex: 1 }} />
            {openExamples.includes(ex.label) && <span style={{ color: "#5a6172", fontSize: 10 }}>open</span>}
          </button>
        ))
      )}
    </div>
  );
}

// Read-only viewer for a single example — copy the whole thing, or select parts.
function ExampleViewer({ ex }: { ex: JsExample }) {
  const [copied, setCopied] = useState(false);
  const extensions = useMemo(() => [javascript()], []);
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, color: "#e6e8eb" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", borderBottom: "1px solid #2c313c", flexShrink: 0 }}>
        <BookOpen size={13} style={{ color: "#8892a0" }} />
        <span style={{ fontSize: 12, fontWeight: 600 }}>{ex.label}</span>
        {ex.desc && <span style={{ color: "#8892a0", fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{ex.desc}</span>}
        <span style={{ color: "#5a6172", fontSize: 10 }}>read-only — select to copy</span>
        <span style={{ flex: 1 }} />
        <button
          onClick={() => {
            void navigator.clipboard?.writeText(ex.source).then(() => { setCopied(true); window.setTimeout(() => setCopied(false), 1200); }).catch(() => {});
          }}
          style={{ ...btn, ...(copied ? btnPrimary : {}) }}
        >
          <Copy size={13} /> {copied ? "Copied" : "Copy all"}
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, overflow: "hidden" }}>
        <CodeMirror value={ex.source} height="100%" theme={oneDark} editable={false} extensions={extensions} style={{ height: "100%", fontSize: 13 }} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-component editor: source + live log + Save & Assign for one jsLogic.
// ---------------------------------------------------------------------------
function JsScriptEditor({ componentUid, active, node, ctx, service, serviceState, onDirty }: {
  componentUid: number;
  active: boolean;
  node: WidgetProps["node"];
  ctx: RenderCtx;
  service: Component | null;
  serviceState: "loading" | "ok" | "missing";
  onDirty: (uid: number, dirty: boolean) => void;
}) {
  // `ctx` is rebuilt each host render — read it through a ref so effects don't
  // re-run (and re-load, clobbering edits) on every canvas re-render.
  const ctxRef = useRef(ctx);
  ctxRef.current = ctx;

  const loadAction = (node.loadAction as string) ?? "getScript";
  const saveAction = node.action?.name ?? "putScript";
  const sourceKey = (node.sourceKey as string) ?? "source";
  const scriptIdProp = (node.scriptIdProp as string) ?? "scriptId";
  const scriptIdParam = (node.scriptIdParam as string) ?? "scriptId";
  const scriptIdSetAction = (node.scriptIdSetAction as string) ?? "setScript";
  const availProp = (node.availableScriptsProp as string) ?? "availableScripts";
  const listAction = (node.listAction as string) ?? "listScripts";
  const apiAction = (node.apiAction as string) ?? "getApi";
  const logProp = node.bind?.prop ?? "log";

  // The jsLogic may be off-canvas (opened from the global list), so the
  // structural store won't hold it — fetch it once for its prop uids.
  const structComp = useStructural((s) => s.components.get(componentUid));
  const [fetched, setFetched] = useState<Component | null>(null);
  useEffect(() => {
    if (structComp) return;
    let alive = true;
    getNodeByUid(componentUid, { depth: 0 }).then((r) => { if (alive) setFetched(r.nodes[0] ?? null); }).catch(() => {});
    return () => { alive = false; };
  }, [componentUid, structComp]);
  const comp = structComp ?? fetched;

  const serviceUid = service?.uid ?? null;
  const scriptIdUid = comp?.properties[scriptIdProp]?.uid;
  const logUid = comp?.properties[logProp]?.uid;
  const availUid = service?.properties[availProp]?.uid;

  // Only the ACTIVE tab subscribes (prop-subscription replaces the whole set).
  useEffect(() => {
    if (!active) return;
    const sub = ctxRef.current.subscribeProps;
    if (!sub) return;
    const uids = [scriptIdUid, logUid, availUid].filter((x): x is number => x != null);
    if (uids.length === 0) return;
    return sub(uids);
  }, [active, scriptIdUid, logUid, availUid]);

  const liveScriptId = useValues((s) => (scriptIdUid != null ? s.values.get(scriptIdUid) : undefined));
  const [optimisticBound, setOptimisticBound] = useState<string | null>(null);
  const streamedBound = asStr(liveScriptId) || asStr(comp?.properties[scriptIdProp]?.value);
  const boundId = optimisticBound ?? streamedBound;
  useEffect(() => {
    if (optimisticBound != null && streamedBound === optimisticBound) setOptimisticBound(null);
  }, [streamedBound, optimisticBound]);

  const [editingId, setEditingId] = useState<string | null>(null);
  const scriptId = editingId ?? boundId;
  const assigned = scriptId !== "" && scriptId === boundId;

  const liveAvail = useValues((s) => (availUid != null ? s.values.get(availUid) : undefined));
  const [listFallback, setListFallback] = useState<string[]>([]);
  useEffect(() => {
    const call = ctxRef.current.callAction;
    if (!active || serviceUid == null || !call || availUid != null) return;
    let alive = true;
    call(serviceUid, listAction, {}).then((r) => { if (alive) setListFallback(splitIds(asStr(r?.ids))); }).catch(() => {});
    return () => { alive = false; };
  }, [active, serviceUid, availUid, listAction]);
  const [localIds, setLocalIds] = useState<string[]>([]);
  const available = useMemo(() => {
    const fromProp = splitIds(asStr(liveAvail) || asStr(service?.properties[availProp]?.value));
    const base = fromProp.length ? fromProp : listFallback;
    return Array.from(new Set([...base, ...localIds])).sort();
  }, [liveAvail, service, availProp, listFallback, localIds]);

  // Autocompletion symbols from the service's getApi (cached per service). Read
  // via a ref so the editor's extensions stay stable while symbols load in.
  const symbolsRef = useRef<JsSymbol[]>([]);
  useEffect(() => {
    const call = ctxRef.current.callAction;
    if (serviceUid == null || !call) return;
    let alive = true;
    loadJsApi(call, serviceUid, apiAction).then((s) => { if (alive) symbolsRef.current = s; }).catch(() => {});
    return () => { alive = false; };
  }, [serviceUid, apiAction]);
  const cmExtensions = useMemo(() => [javascript(), autocompletion({ override: [jsCompletionSource(symbolsRef)] })], []);

  const [code, setCode] = useState("");
  const [dirty, setDirty] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  useEffect(() => { onDirty(componentUid, dirty); }, [componentUid, dirty, onDirty]);

  const [logLines, setLogLines] = useState<string[]>([]);
  const pushLog = useCallback((line: string) => {
    setLogLines((prev) => {
      const next = prev.concat(line.split("\n"));
      return next.length > MAX_LOG_LINES ? next.slice(next.length - MAX_LOG_LINES) : next;
    });
  }, []);

  const ready = serviceUid != null && !!ctx.callAction;
  const canEdit = ready && scriptId !== "";
  const canSave = canEdit && !saving;

  const loadToken = useRef(0);
  const load = useCallback(() => {
    const call = ctxRef.current.callAction;
    if (serviceUid == null || !call || scriptId === "") return;
    const token = ++loadToken.current;
    setLoading(true);
    call(serviceUid, loadAction, { [scriptIdParam]: scriptId })
      .then((ret) => {
        if (token !== loadToken.current) return;
        const err = asStr(ret?.error);
        if (err) { pushLog(`[load] ${scriptId}: ${err}`); }
        else { const src = asStr(ret?.[sourceKey]); setCode(src); setDirty(false); pushLog(`[load] ${scriptId} (${src.length} chars)`); }
        setLoading(false);
      })
      .catch((e: unknown) => { if (token !== loadToken.current) return; pushLog(`[load] ${scriptId}: ${e instanceof Error ? e.message : String(e)}`); setLoading(false); });
  }, [serviceUid, loadAction, scriptId, scriptIdParam, sourceKey, pushLog]);
  useEffect(() => { load(); return () => { loadToken.current++; }; }, [load]);

  const assign = useCallback((id: string): Promise<boolean> => {
    const call = ctxRef.current.callAction;
    if (!call || id === "") return Promise.resolve(false);
    pushLog(`[assign] setScript ${id}…`);
    return call(componentUid, scriptIdSetAction, { [scriptIdParam]: id })
      .then((ret) => {
        const err = asStr(ret?.error);
        if (err) { pushLog(`[assign] ${id}: ${err}`); return false; }
        setOptimisticBound(id); setEditingId(null); pushLog(`[assign] ${id}: ok`); return true;
      })
      .catch((e: unknown) => { pushLog(`[assign] ${id}: ${e instanceof Error ? e.message : String(e)}`); return false; });
  }, [componentUid, scriptIdSetAction, scriptIdParam, pushLog]);

  const save = async () => {
    if (!canSave) return;
    const id = scriptId;
    setSaving(true);
    try {
      if (dirty) {
        pushLog(`[save] putScript ${id}…`);
        const ret = await ctxRef.current.callAction!(serviceUid!, saveAction, { [scriptIdParam]: id, [sourceKey]: code as FlexValue });
        const ok = ret?.ok === true || asStr(ret?.ok) === "true";
        const err = asStr(ret?.error);
        if (!ok || err) { pushLog(`[save] ${id}: ${err || "rejected"}`); return; }
        setDirty(false); pushLog(`[save] ${id}: ok`);
      }
      await assign(id);
    } finally { setSaving(false); }
  };

  // Ctrl/Cmd+S → Save & Assign (deploy), not the browser's save dialog. Only the
  // active tab handles it; a ref keeps the latest `save` without re-binding.
  const saveRef = useRef(save);
  saveRef.current = save;
  useEffect(() => {
    if (!active) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        e.stopPropagation();
        void saveRef.current();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [active]);

  const [creating, setCreating] = useState(false);
  const [newId, setNewId] = useState("");
  const createScript = () => {
    const id = newId.trim();
    const call = ctxRef.current.callAction;
    if (serviceUid == null || !call || id === "") return;
    pushLog(`[create] putScript ${id} (seed template)…`);
    call(serviceUid, saveAction, { [scriptIdParam]: id, [sourceKey]: "" as FlexValue })
      .then((ret) => {
        const err = asStr(ret?.error);
        if (err) { pushLog(`[create] ${id}: ${err}`); return; }
        pushLog(`[create] ${id}: ok`);
        setCreating(false); setNewId("");
        setLocalIds((prev) => (prev.includes(id) ? prev : [...prev, id]));
        setEditingId(id);
      })
      .catch((e: unknown) => pushLog(`[create] ${id}: ${e instanceof Error ? e.message : String(e)}`));
  };

  // Stream the jsLogic's log output (only meaningful while active/subscribed).
  const liveLog = useValues((s) => (logUid != null ? s.values.get(logUid) : undefined));
  const lastLive = useRef<string | null>(null);
  useEffect(() => {
    if (liveLog == null) return;
    const s = asStr(liveLog);
    if (s === lastLive.current) return;
    lastLive.current = s;
    if (s) pushLog(s);
  }, [liveLog, pushLog]);

  const logRef = useRef<HTMLDivElement>(null);
  useEffect(() => { const el = logRef.current; if (el) el.scrollTop = el.scrollHeight; }, [logLines]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, color: "#e6e8eb" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", borderBottom: "1px solid #2c313c", flexShrink: 0, flexWrap: "wrap" }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>{comp?.name ?? `#${componentUid}`}</span>
        <span style={{ color: "#8892a0", fontSize: 11 }}>script</span>
        <select value={scriptId || ""} onChange={(e) => setEditingId(e.target.value)} disabled={!ready} style={inp}>
          {scriptId === "" && <option value="" disabled>— select —</option>}
          {Array.from(new Set([scriptId, ...available].filter(Boolean))).map((id) => <option key={id} value={id}>{id}</option>)}
        </select>
        {creating ? (
          <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <input autoFocus value={newId} onChange={(e) => setNewId(e.target.value)} placeholder="new script id"
              onKeyDown={(e) => { if (e.key === "Enter") createScript(); if (e.key === "Escape") { setCreating(false); setNewId(""); } }}
              style={{ ...inp, width: 140 }} />
            <button onClick={createScript} disabled={!newId.trim()} style={{ ...btn, ...btnPrimary, opacity: newId.trim() ? 1 : 0.45 }}>Create</button>
            <button onClick={() => { setCreating(false); setNewId(""); }} style={btn}>Cancel</button>
          </span>
        ) : (
          <button onClick={() => setCreating(true)} disabled={!ready} title="Create a new script" style={btn}><Plus size={13} /> New</button>
        )}
        {serviceState === "missing" && <span style={{ color: "#e0707a", fontSize: 11 }}>jsScriptStore not found</span>}
        {loading && <span style={{ color: "#5a6172", fontSize: 11 }}>loading…</span>}
        {scriptId !== "" && (assigned
          ? <span style={{ color: "#6a9a6a", fontSize: 11 }}>● assigned</span>
          : <span style={{ color: "#e0b341", fontSize: 11 }}>not assigned — Save to assign</span>)}
        {dirty && <span style={{ color: "#e0b341", fontSize: 11 }}>● unsaved</span>}
        <span style={{ flex: 1 }} />
        <button onClick={load} disabled={!canEdit || loading} style={{ ...btn, opacity: !canEdit || loading ? 0.45 : 1 }}><RefreshCw size={13} /> Reload</button>
        <button onClick={save} disabled={!canSave} title="Save the source and assign this script to the component" style={{ ...btn, ...btnPrimary, opacity: !canSave ? 0.45 : 1 }}>
          <Save size={13} /> {saving ? "Saving…" : assigned ? "Save" : "Save & Assign"}
        </button>
      </div>

      <div style={{ flex: 1, minHeight: 0, overflow: "hidden" }}>
        {scriptId === "" ? (
          <div style={{ padding: 16, color: "#8892a0", fontSize: 12 }}>
            No script open. Pick one from the dropdown above, or{" "}
            <button onClick={() => setCreating(true)} style={{ ...btn, display: "inline-flex", padding: "2px 8px" }}>create a new script</button>.
            {boundId === "" && <span> <b style={{ color: "#cbd3e0" }}>{comp?.name ?? `#${componentUid}`}</b> has no script assigned yet.</span>}
          </div>
        ) : (
          <CodeMirror value={code} height="100%" theme={oneDark} extensions={cmExtensions} onChange={(v) => { setCode(v); setDirty(true); }} style={{ height: "100%", fontSize: 13 }} />
        )}
      </div>

      <div style={{ flexShrink: 0, borderTop: "1px solid #2c313c", display: "flex", flexDirection: "column", height: 160 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "4px 10px", borderBottom: "1px solid #21262f", flexShrink: 0 }}>
          <span style={{ color: "#8892a0", fontSize: 10, textTransform: "uppercase", letterSpacing: 0.4 }}>Debug log</span>
          <span style={{ color: "#5a6172", fontSize: 10 }}>{logLines.length}</span>
          <span style={{ flex: 1 }} />
          <button onClick={() => setLogLines([])} title="Clear log" style={{ ...btn, padding: "2px 8px" }}><Trash2 size={12} /> clear</button>
        </div>
        <div ref={logRef} style={{ flex: 1, overflowY: "auto", padding: "4px 10px", fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 11, lineHeight: 1.5, whiteSpace: "pre-wrap", color: "#cbd3e0", background: "#0f1115" }}>
          {logLines.length === 0 ? <span style={{ color: "#5a6172" }}>{logUid == null ? "no log output" : "waiting for output…"}</span> : logLines.map((l, i) => <div key={i}>{l}</div>)}
        </div>
      </div>
    </div>
  );
}

const inp: React.CSSProperties = { background: "#0f1115", color: "#e6e8eb", border: "1px solid #2c313c", borderRadius: 4, padding: "3px 7px", fontSize: 12, outline: "none" };
const btn: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, padding: "4px 10px", fontSize: 11, color: "#cbd3e0", background: "#2c313c", border: "1px solid #3a4150", borderRadius: 4, cursor: "pointer" };
const btnPrimary: React.CSSProperties = { background: "#2c3a55", borderColor: "#3b5388", color: "#cfe0ff" };
const iconBtn: React.CSSProperties = { display: "inline-flex", alignItems: "center", marginLeft: 2, padding: 1, borderRadius: 3, color: "#8892a0" };

registerWidget("jsEditor", JsEditorPanel);

export default JsEditorPanel;
