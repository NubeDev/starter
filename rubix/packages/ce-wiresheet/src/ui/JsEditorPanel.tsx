// `jsEditor` widget — edit the JS a `jsLogic` component runs (NubeIO-js manifest).
//
// Data model (all names come from the descriptor; defaults match the manifest):
//   - jsLogic.scriptId  — OUTPUT reporting the bound script id. Changed via the
//                         `setScript({ scriptId })` action ON the jsLogic.
//   - jsLogic.log       — OUTPUT string stream → the debug-log pane.
//   - jsScriptStore     — singleton service (role "js.service"):
//       availableScripts (OUTPUT) → the dropdown of script ids
//       getScript({scriptId})→{source}, putScript({scriptId,source})→{ok,error}
//
// Flow: pick a jsLogic (globally, via the picker) → its scriptId names a script
// in the store → load/edit/save the source. Empty/!set → choose from the
// dropdown or create a new id; selecting binds it with setScript.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import { autocompletion } from "@codemirror/autocomplete";
import { oneDark } from "@codemirror/theme-one-dark";
import { Save, RefreshCw, Trash2, Plus } from "lucide-react";
import { useStructural, useValues } from "../lib/store";
import type { Component, FlexValue } from "../lib/engine-types";
import { resolveJsStore } from "./jsScriptStore";
import { registerWidget, type WidgetProps } from "./registry";

const MAX_LOG_LINES = 500;
const asStr = (v: unknown): string => (typeof v === "string" ? v : v == null ? "" : String(v));
const splitIds = (s: string): string[] => s.split(",").map((x) => x.trim()).filter(Boolean);

function JsEditorPanel({ node, ctx }: WidgetProps) {
  const uid = ctx.componentUid;
  const comp = useStructural((s) => (uid != null ? s.components.get(uid) : undefined));

  // `ctx` (and its callAction/subscribeProps) is a fresh object on every render
  // of the host, so effects must NOT depend on it — otherwise they'd re-run on
  // every canvas re-render (node moves, streaming values), spuriously re-loading
  // the script and clobbering unsaved edits. Read it through a ref instead and
  // key effects on the real data (scriptId / serviceUid / action names).
  const ctxRef = useRef(ctx);
  ctxRef.current = ctx;

  // Descriptor config (defaults = NubeIO-js manifest).
  const serviceType = (node.serviceType as string) ?? "jsScriptStore";
  const loadAction = (node.loadAction as string) ?? "getScript";
  const saveAction = node.action?.name ?? "putScript";
  const sourceKey = (node.sourceKey as string) ?? "source";
  const scriptIdProp = (node.scriptIdProp as string) ?? "scriptId";
  const scriptIdParam = (node.scriptIdParam as string) ?? "scriptId";
  const scriptIdSetAction = (node.scriptIdSetAction as string) ?? "setScript";
  const availProp = (node.availableScriptsProp as string) ?? "availableScripts";
  const listAction = (node.listAction as string) ?? "listScripts";
  const logProp = node.bind?.prop ?? "log";

  // Resolve the jsScriptStore singleton (its Component carries the availableScripts uid).
  const [service, setService] = useState<Component | null>(null);
  const [serviceState, setServiceState] = useState<"loading" | "ok" | "missing">("loading");
  useEffect(() => {
    let alive = true;
    setServiceState("loading");
    resolveJsStore(serviceType)
      .then((svc) => {
        if (!alive) return;
        if (svc) { setService(svc); setServiceState("ok"); }
        else setServiceState("missing");
      })
      .catch(() => alive && setServiceState("missing"));
    return () => { alive = false; };
  }, [serviceType]);
  const serviceUid = service?.uid ?? null;

  // Prop uids to stream: the jsLogic's scriptId + log (it may be off-canvas after a
  // global pick) and the service's availableScripts. subscribeProps replaces the
  // whole set per call, so subscribe all three in ONE call.
  const scriptIdUid = comp?.properties[scriptIdProp]?.uid;
  const logUid = comp?.properties[logProp]?.uid;
  const availUid = service?.properties[availProp]?.uid;
  useEffect(() => {
    const sub = ctxRef.current.subscribeProps;
    if (!sub) return;
    const uids = [scriptIdUid, logUid, availUid].filter((x): x is number => x != null);
    if (uids.length === 0) return;
    return sub(uids);
  }, [scriptIdUid, logUid, availUid]);

  // The component's CURRENTLY-ASSIGNED script id (the scriptId OUTPUT) — live,
  // with an optimistic override right after Save assigns, before the stream
  // catches up.
  const liveScriptId = useValues((s) => (scriptIdUid != null ? s.values.get(scriptIdUid) : undefined));
  const [optimisticBound, setOptimisticBound] = useState<string | null>(null);
  const streamedBound = asStr(liveScriptId) || asStr(comp?.properties[scriptIdProp]?.value);
  const boundId = optimisticBound ?? streamedBound;
  useEffect(() => {
    if (optimisticBound != null && streamedBound === optimisticBound) setOptimisticBound(null);
  }, [streamedBound, optimisticBound]);

  // Which script is open in the EDITOR. Defaults to the assigned one; the dropdown
  // changes only this (it does NOT bind — assignment happens on Save). Reset on
  // component switch so the editor follows the newly-selected component's script.
  const [editingId, setEditingId] = useState<string | null>(null);
  useEffect(() => { setEditingId(null); }, [uid]);
  const scriptId = editingId ?? boundId; // the id being viewed/edited/loaded
  const assigned = scriptId !== "" && scriptId === boundId;

  // Available script ids — from the service's availableScripts output (live),
  // falling back to a listScripts call if that prop isn't streaming.
  const liveAvail = useValues((s) => (availUid != null ? s.values.get(availUid) : undefined));
  const [listFallback, setListFallback] = useState<string[]>([]);
  useEffect(() => {
    const call = ctxRef.current.callAction;
    if (serviceUid == null || !call || availUid != null) return; // prop covers it
    let alive = true;
    call(serviceUid, listAction, {})
      .then((r) => { if (alive) setListFallback(splitIds(asStr(r?.ids))); })
      .catch(() => {});
    return () => { alive = false; };
  }, [serviceUid, availUid, listAction]);
  // Optimistically-created ids — shown in the dropdown immediately, before the
  // availableScripts stream catches up (so a new script doesn't require a reload).
  const [localIds, setLocalIds] = useState<string[]>([]);
  const available = useMemo(() => {
    const fromProp = splitIds(asStr(liveAvail) || asStr(service?.properties[availProp]?.value));
    const base = fromProp.length ? fromProp : listFallback;
    return Array.from(new Set([...base, ...localIds])).sort();
  }, [liveAvail, service, availProp, listFallback, localIds]);

  // ---- source editor state ----
  const [code, setCode] = useState("");
  const [dirty, setDirty] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  const [logLines, setLogLines] = useState<string[]>([]);
  const pushLog = useCallback((line: string) => {
    setLogLines((prev) => {
      const next = prev.concat(line.split("\n"));
      return next.length > MAX_LOG_LINES ? next.slice(next.length - MAX_LOG_LINES) : next;
    });
  }, []);

  const ready = serviceUid != null && !!ctx.callAction;
  const canEdit = ready && scriptId !== "";   // a script is open for load/edit
  const canSave = canEdit && !saving;          // Save persists (if changed) + assigns

  // Load the source for the current scriptId from the store.
  const loadToken = useRef(0);
  const load = useCallback(() => {
    if (serviceUid == null || !ctx.callAction || scriptId === "") return;
    const token = ++loadToken.current;
    setLoading(true);
    ctx.callAction(serviceUid, loadAction, { [scriptIdParam]: scriptId })
      .then((ret) => {
        if (token !== loadToken.current) return;
        const err = asStr(ret?.error);
        if (err) { pushLog(`[load] ${scriptId}: ${err}`); }
        else {
          const src = asStr(ret?.[sourceKey]);
          setCode(src); setDirty(false);
          pushLog(`[load] ${scriptId} (${src.length} chars)`);
        }
        setLoading(false);
      })
      .catch((e: unknown) => {
        if (token !== loadToken.current) return;
        pushLog(`[load] ${scriptId}: ${e instanceof Error ? e.message : String(e)}`);
        setLoading(false);
      });
  }, [serviceUid, ctx, loadAction, scriptId, scriptIdParam, sourceKey, pushLog]);
  useEffect(() => { load(); return () => { loadToken.current++; }; }, [load]);

  // Assign the open script to this component via setScript (scriptId is an output).
  const assign = useCallback((id: string): Promise<boolean> => {
    if (uid == null || !ctx.callAction || id === "") return Promise.resolve(false);
    pushLog(`[assign] setScript ${id}…`);
    return ctx.callAction(uid, scriptIdSetAction, { [scriptIdParam]: id })
      .then((ret) => {
        const err = asStr(ret?.error);
        if (err) { pushLog(`[assign] ${id}: ${err}`); return false; }
        setOptimisticBound(id); setEditingId(null);
        pushLog(`[assign] ${id}: ok`);
        return true;
      })
      .catch((e: unknown) => { pushLog(`[assign] ${id}: ${e instanceof Error ? e.message : String(e)}`); return false; });
  }, [uid, ctx, scriptIdSetAction, scriptIdParam, pushLog]);

  // Save = persist the source (if changed) AND assign the script to this
  // component, so one click both saves and makes the component run it.
  const save = async () => {
    if (!canSave) return;
    const id = scriptId;
    setSaving(true);
    try {
      if (dirty) {
        pushLog(`[save] putScript ${id}…`);
        const ret = await ctx.callAction!(serviceUid!, saveAction, { [scriptIdParam]: id, [sourceKey]: code as FlexValue });
        const ok = ret?.ok === true || asStr(ret?.ok) === "true";
        const err = asStr(ret?.error);
        if (!ok || err) { pushLog(`[save] ${id}: ${err || "rejected"}`); return; }
        setDirty(false); pushLog(`[save] ${id}: ok`);
      }
      await assign(id);
    } finally {
      setSaving(false);
    }
  };

  // Create a new script: putScript with NO source — the ext seeds the default
  // template + compiles. We then open it (editingId) so the load below fetches
  // the seeded template via getScript and pre-fills the editor.
  const [creating, setCreating] = useState(false);
  const [newId, setNewId] = useState("");
  const createScript = () => {
    const id = newId.trim();
    if (serviceUid == null || !ctx.callAction || id === "") return;
    pushLog(`[create] putScript ${id} (seed template)…`);
    ctx.callAction(serviceUid, saveAction, { [scriptIdParam]: id, [sourceKey]: "" as FlexValue })
      .then((ret) => {
        const err = asStr(ret?.error);
        if (err) { pushLog(`[create] ${id}: ${err}`); return; }
        pushLog(`[create] ${id}: ok`);
        setCreating(false); setNewId("");
        setLocalIds((prev) => (prev.includes(id) ? prev : [...prev, id])); // show in dropdown now
        setEditingId(id); // open it → load() fetches the seeded template
      })
      .catch((e: unknown) => pushLog(`[create] ${id}: ${e instanceof Error ? e.message : String(e)}`));
  };

  // Stream the jsLogic's log output into the pane.
  const liveLog = useValues((s) => (logUid != null ? s.values.get(logUid) : undefined));
  const lastLive = useRef<string | null>(null);
  useEffect(() => { lastLive.current = null; setLogLines([]); }, [uid]);
  useEffect(() => {
    if (liveLog == null) return;
    const s = asStr(liveLog);
    if (s === lastLive.current) return;
    lastLive.current = s;
    if (s) pushLog(s);
  }, [liveLog, pushLog]);

  const logRef = useRef<HTMLDivElement>(null);
  useEffect(() => { const el = logRef.current; if (el) el.scrollTop = el.scrollHeight; }, [logLines]);

  if (uid == null || !comp) {
    return <div style={{ padding: 12, color: "#5a6172", fontSize: 12 }}>Select a jsLogic component.</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, color: "#e6e8eb" }}>
      {/* header: component + script selector + actions */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", borderBottom: "1px solid #2c313c", flexShrink: 0, flexWrap: "wrap" }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>{comp.name || comp.type}</span>
        <span style={{ color: "#8892a0", fontSize: 11 }}>script</span>
        <select
          value={scriptId || ""}
          onChange={(e) => setEditingId(e.target.value)}
          disabled={!ready}
          style={inp}
        >
          {scriptId === "" && <option value="" disabled>— select —</option>}
          {Array.from(new Set([scriptId, ...available].filter(Boolean))).map((id) => (
            <option key={id} value={id}>{id}</option>
          ))}
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
          <button onClick={() => setCreating(true)} disabled={serviceState !== "ok"} title="Create a new script" style={btn}>
            <Plus size={13} /> New
          </button>
        )}
        {serviceState === "missing" && <span style={{ color: "#e0707a", fontSize: 11 }}>jsScriptStore not found</span>}
        {loading && <span style={{ color: "#5a6172", fontSize: 11 }}>loading…</span>}
        {scriptId !== "" && (assigned
          ? <span style={{ color: "#6a9a6a", fontSize: 11 }}>● assigned</span>
          : <span style={{ color: "#e0b341", fontSize: 11 }}>not assigned — Save to assign</span>)}
        {dirty && <span style={{ color: "#e0b341", fontSize: 11 }}>● unsaved</span>}
        <span style={{ flex: 1 }} />
        <button onClick={load} disabled={!canEdit || loading} style={{ ...btn, opacity: !canEdit || loading ? 0.45 : 1 }}>
          <RefreshCw size={13} /> Reload
        </button>
        <button
          onClick={save}
          disabled={!canSave}
          title="Save the source and assign this script to the component"
          style={{ ...btn, ...btnPrimary, opacity: !canSave ? 0.45 : 1 }}
        >
          <Save size={13} /> {saving ? "Saving…" : assigned ? "Save" : "Save & Assign"}
        </button>
      </div>

      {/* editor (or empty-state prompt when no script is bound) */}
      <div style={{ flex: 1, minHeight: 0, overflow: "hidden", position: "relative" }}>
        {scriptId === "" ? (
          <div style={{ padding: 16, color: "#8892a0", fontSize: 12 }}>
            No script open. Pick one from the dropdown above, or{" "}
            <button onClick={() => setCreating(true)} style={{ ...btn, display: "inline-flex", padding: "2px 8px" }}>create a new script</button>.
            {boundId === "" && <span> <b style={{ color: "#cbd3e0" }}>{comp.name || comp.type}</b> has no script assigned yet.</span>}
          </div>
        ) : (
          <CodeMirror
            value={code}
            height="100%"
            theme={oneDark}
            extensions={[javascript(), autocompletion()]}
            onChange={(v) => { setCode(v); setDirty(true); }}
            style={{ height: "100%", fontSize: 13 }}
          />
        )}
      </div>

      {/* debug log (jsLogic.log stream + action results) */}
      <div style={{ flexShrink: 0, borderTop: "1px solid #2c313c", display: "flex", flexDirection: "column", height: 160 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "4px 10px", borderBottom: "1px solid #21262f", flexShrink: 0 }}>
          <span style={{ color: "#8892a0", fontSize: 10, textTransform: "uppercase", letterSpacing: 0.4 }}>Debug log</span>
          <span style={{ color: "#5a6172", fontSize: 10 }}>{logLines.length}</span>
          <span style={{ flex: 1 }} />
          <button onClick={() => setLogLines([])} title="Clear log" style={{ ...btn, padding: "2px 8px" }}>
            <Trash2 size={12} /> clear
          </button>
        </div>
        <div ref={logRef} style={{ flex: 1, overflowY: "auto", padding: "4px 10px", fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 11, lineHeight: 1.5, whiteSpace: "pre-wrap", color: "#cbd3e0", background: "#0f1115" }}>
          {logLines.length === 0 ? (
            <span style={{ color: "#5a6172" }}>{logUid == null ? "no log output" : "waiting for output…"}</span>
          ) : (
            logLines.map((l, i) => <div key={i}>{l}</div>)
          )}
        </div>
      </div>
    </div>
  );
}

const inp: React.CSSProperties = { background: "#0f1115", color: "#e6e8eb", border: "1px solid #2c313c", borderRadius: 4, padding: "3px 7px", fontSize: 12, outline: "none" };
const btn: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, padding: "4px 10px", fontSize: 11, color: "#cbd3e0", background: "#2c313c", border: "1px solid #3a4150", borderRadius: 4, cursor: "pointer" };
const btnPrimary: React.CSSProperties = { background: "#2c3a55", borderColor: "#3b5388", color: "#cfe0ff" };

registerWidget("jsEditor", JsEditorPanel);

export default JsEditorPanel;
