// The `schedule` widget — interactive weekly calendar + exceptions list for the
// NubeIO `schedule` component. Edits the component's `config` JSON (weekly +
// calendar entries); Save commits via the `setSchedule` action and surfaces its
// {ok,error}. Shows live outputs (out / active / nextChange) and a manual timer
// (startTimer / cancelTimer + timerEnd). cron entries are out of v1 scope.
//
// config schema (engine-owned semantics; precedence: calendar > weekly > default):
//   { "default": false, "entries": [
//       { "type":"weekly",   "name":"Occupied", "days":["mon",…], "start":"08:00", "end":"18:00", "value":true },
//       { "type":"calendar", "name":"Holiday",  "from":"2026-12-25", "to":"2026-12-26", "value":false } ] }

import { useEffect, useMemo, useRef, useState } from "react";
import { useStructural, useValues } from "../lib/store";
import { Plus, Save, RotateCcw, Trash2, Play, Square, CalendarPlus } from "lucide-react";
import { registerWidget, type WidgetProps } from "./registry";
import type { Component } from "../lib/engine-types";

type DayKey = "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun";
const DAYS: DayKey[] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const DAY_LABEL: Record<DayKey, string> = { mon: "Mon", tue: "Tue", wed: "Wed", thu: "Thu", fri: "Fri", sat: "Sat", sun: "Sun" };

interface WeeklyEntry { type: "weekly"; name?: string; days: DayKey[]; start: string; end: string; value: boolean }
interface CalendarEntry { type: "calendar"; name?: string; from: string; to: string; value: boolean }
type Entry = WeeklyEntry | CalendarEntry;
interface Schedule { default?: boolean; entries: Entry[] }
/** A partial patch over any entry's editable fields (no discriminant). */
type EntryPatch = Partial<Omit<WeeklyEntry, "type"> & Omit<CalendarEntry, "type">>;

const SAMPLE: Schedule = {
  default: false,
  entries: [
    { type: "weekly", name: "Occupied", days: ["mon", "tue", "wed", "thu", "fri"], start: "08:00", end: "18:00", value: true },
    { type: "weekly", name: "Weekend", days: ["sat", "sun"], start: "09:00", end: "14:00", value: true },
    { type: "calendar", name: "Holiday Shutdown", from: "2026-12-25", to: "2026-12-26", value: false },
  ],
};

const HOUR_PX = 26;
const GRID_H = 24 * HOUR_PX;
const HEADER_H = 28;
const SNAP = 15;
const clamp = (n: number, a: number, b: number) => Math.max(a, Math.min(b, n));
const toMin = (hhmm: string) => { const [h, m] = hhmm.split(":").map(Number); return (h || 0) * 60 + (m || 0); };
const toHHMM = (min: number) => {
  const v = clamp(Math.round(min / SNAP) * SNAP, 0, 1440);
  return `${String(Math.floor(v / 60)).padStart(2, "0")}:${String(v % 60).padStart(2, "0")}`;
};
const fmtEpoch = (v: unknown) =>
  typeof v === "number" && v > 0 ? new Date(v * 1000).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" }) : "—";

type Drag =
  | { kind: "create"; day: DayKey; fromMin: number; toMin: number }
  | { kind: "move"; index: number; grabMin: number; dur: number }
  | { kind: "resize"; index: number };

function parse(raw: string | undefined): Schedule | null {
  if (!raw) return null;
  try {
    const o = JSON.parse(raw);
    if (!o || !Array.isArray(o.entries)) return null;
    return o as Schedule;
  } catch {
    return null;
  }
}

function SchedulePanel({ node, ctx }: WidgetProps) {
  const propName = node.bind?.prop ?? "config";
  const comp = useStructural((s) => (ctx.componentUid != null ? s.components.get(ctx.componentUid) : undefined));
  const uid = comp ? comp.properties[propName]?.uid : undefined;
  const live = useValues((s) => (uid != null ? s.values.get(uid) : undefined));
  const sourceStr = typeof live === "string" && live ? live : undefined;

  // When bound to a real component, an empty `config` means an empty schedule —
  // only fall back to the sample when there's no component at all (dev/stub).
  const seedFor = (s: string | undefined): Schedule =>
    parse(s) ?? (ctx.componentUid != null ? { default: false, entries: [] } : SAMPLE);
  const [draft, setDraft] = useState<Schedule>(() => seedFor(sourceStr));
  const [dirty, setDirty] = useState(false);
  const [sel, setSel] = useState<number | null>(null); // selected weekly entry index
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [timerSec, setTimerSec] = useState(600);
  const seeded = useRef(sourceStr);

  useEffect(() => {
    if (!dirty && sourceStr !== seeded.current) {
      seeded.current = sourceStr;
      setDraft(seedFor(sourceStr));
      setSel(null);
    }
  }, [sourceStr, dirty]);

  const mutate = (fn: (d: Schedule) => Schedule) => {
    setDraft((d) => fn(structuredClone(d)));
    setDirty(true);
    setSaveErr(null);
  };
  const patchEntry = (i: number, patch: EntryPatch) =>
    mutate((d) => ((d.entries[i] = { ...d.entries[i], ...patch } as Entry), d));

  // --- drag on the weekly grid ---------------------------------------------
  const [drag, setDrag] = useState<Drag | null>(null);
  const colRef = useRef<HTMLDivElement | null>(null);
  const geom = useRef({ top: 0, height: GRID_H });
  const moved = useRef(false);
  const captureGeom = () => {
    const r = colRef.current?.getBoundingClientRect();
    if (r) geom.current = { top: r.top, height: r.height };
  };
  const minAt = (clientY: number) => clamp(Math.round(((clientY - geom.current.top) / geom.current.height) * 1440 / SNAP) * SNAP, 0, 1440);

  useEffect(() => {
    if (!drag) return;
    const onMove = (ev: PointerEvent) => {
      moved.current = true;
      const m = minAt(ev.clientY);
      if (drag.kind === "create") setDrag((d) => (d && d.kind === "create" ? { ...d, toMin: m } : d));
      else if (drag.kind === "move") {
        const start = clamp(m - drag.grabMin, 0, 1440 - drag.dur);
        patchEntry(drag.index, { start: toHHMM(start), end: toHHMM(start + drag.dur) });
      } else {
        const e = draft.entries[drag.index] as WeeklyEntry;
        patchEntry(drag.index, { end: toHHMM(Math.max(m, toMin(e.start) + SNAP)) });
      }
    };
    const onUp = () => {
      if (drag.kind === "create") {
        const a = Math.min(drag.fromMin, drag.toMin);
        const b = Math.max(drag.fromMin, drag.toMin);
        if (b - a >= SNAP) {
          const idx = draft.entries.length;
          mutate((d) => ((d.entries.push({ type: "weekly", days: [drag.day], start: toHHMM(a), end: toHHMM(b), value: true }), d)));
          setSel(idx);
        }
      }
      setDrag(null);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, [drag, draft]);

  const weeklyByDay = useMemo(() => {
    const byDay: Record<DayKey, { i: number; e: WeeklyEntry }[]> = { mon: [], tue: [], wed: [], thu: [], fri: [], sat: [], sun: [] };
    draft.entries.forEach((e, i) => {
      if (e.type === "weekly") e.days.forEach((d) => byDay[d]?.push({ i, e }));
    });
    return byDay;
  }, [draft]);
  const calendarEntries = useMemo(
    () => draft.entries.map((e, i) => ({ i, e })).filter((x): x is { i: number; e: CalendarEntry } => x.e.type === "calendar"),
    [draft],
  );

  const addCalendar = () =>
    mutate((d) => ((d.entries.push({ type: "calendar", name: "Exception", from: "", to: "", value: false }), d)));
  const removeEntry = (i: number) => { mutate((d) => ((d.entries.splice(i, 1)), d)); setSel(null); };

  const save = async () => {
    if (ctx.componentUid == null || !ctx.callAction) { setDirty(false); return; }
    const r = await ctx.callAction(ctx.componentUid, node.action?.name ?? "setSchedule", { json: JSON.stringify(draft) });
    if (r && r.ok === false) setSaveErr(typeof r.error === "string" ? r.error : "save rejected");
    else { setDirty(false); setSaveErr(null); }
  };
  const reset = () => { setDraft(seedFor(sourceStr)); setDirty(false); setSel(null); setSaveErr(null); };

  const canCall = ctx.componentUid != null && !!ctx.callAction;

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", userSelect: "none", fontSize: 12 }}>
      {/* status + timer strip */}
      {comp && (
        <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "5px 8px", borderBottom: "1px solid #2c313c", flexShrink: 0, flexWrap: "wrap" }}>
          <LiveOut comp={comp} />
          <span style={muted}>active <b style={{ color: "#e6e8eb" }}><LiveText comp={comp} prop="active" /></b></span>
          <span style={muted}>next <b style={{ color: "#e6e8eb" }}><LiveTime comp={comp} prop="nextChange" /></b></span>
          <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6 }}>
            <span style={muted}>timer ends <b style={{ color: "#e6e8eb" }}><LiveTime comp={comp} prop="timerEnd" /></b></span>
            <input type="number" value={timerSec} onChange={(e) => setTimerSec(Number(e.target.value))} title="seconds" style={{ ...inp, width: 64 }} />
            <button disabled={!canCall} onClick={() => canCall && ctx.callAction!(ctx.componentUid!, "startTimer", { duration: timerSec })} style={tbBtn}>
              <Play size={12} /> Start
            </button>
            <button disabled={!canCall} onClick={() => canCall && ctx.callAction!(ctx.componentUid!, "cancelTimer", {})} style={tbBtn}>
              <Square size={12} /> Cancel
            </button>
          </span>
        </div>
      )}

      {/* toolbar */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 8px", borderBottom: "1px solid #2c313c", flexShrink: 0 }}>
        <span style={muted}>drag the grid to add · drag a block to move · drag its edge to resize</span>
        <button onClick={addCalendar} style={{ ...tbBtn, marginLeft: "auto" }}>
          <CalendarPlus size={13} /> Exception
        </button>
        <label style={{ display: "flex", alignItems: "center", gap: 5, color: "#8892a0" }}>
          default
          <button onClick={() => mutate((d) => ((d.default = !d.default), d))} style={toggle(!!draft.default)}>
            {draft.default ? "ON" : "OFF"}
          </button>
        </label>
        {saveErr && <span style={{ color: "#e0707a", fontSize: 11 }}>⚠ {saveErr}</span>}
        {dirty && <span style={{ color: "#d8a657", fontSize: 11 }}>● unsaved</span>}
        <button onClick={reset} disabled={!dirty} style={{ ...tbBtn, opacity: dirty ? 1 : 0.4 }}><RotateCcw size={13} /> Reset</button>
        <button onClick={save} disabled={!dirty} style={{ ...tbBtn, ...tbPrimary, opacity: dirty ? 1 : 0.4 }}><Save size={13} /> Save</button>
      </div>

      {/* weekly grid */}
      <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
        <div style={{ display: "flex", minWidth: 520 }}>
          <div style={{ width: 46, flexShrink: 0, paddingTop: HEADER_H }}>
            {Array.from({ length: 24 }, (_, h) => (
              <div key={h} style={{ height: HOUR_PX, fontSize: 9, color: "#5a6172", textAlign: "right", paddingRight: 5, transform: "translateY(-6px)" }}>
                {h === 0 ? "" : `${h}:00`}
              </div>
            ))}
          </div>
          {DAYS.map((d) => (
            <div key={d} style={{ flex: 1, minWidth: 64, borderLeft: "1px solid #23272f" }}>
              <div style={{ height: HEADER_H, display: "flex", alignItems: "center", justifyContent: "center", color: "#cbd3e0", fontWeight: 500, borderBottom: "1px solid #2c313c", position: "sticky", top: 0, background: "#1a1d24", zIndex: 1 }}>
                {DAY_LABEL[d]}
              </div>
              <div
                ref={d === DAYS[0] ? colRef : undefined}
                onPointerDown={(ev) => {
                  if (ev.button !== 0) return;
                  captureGeom();
                  moved.current = false;
                  const m = minAt(ev.clientY);
                  setDrag({ kind: "create", day: d, fromMin: m, toMin: m });
                }}
                style={{ position: "relative", height: GRID_H, cursor: "crosshair", backgroundImage: `repeating-linear-gradient(to bottom, transparent 0, transparent ${HOUR_PX - 1}px, #20242c ${HOUR_PX - 1}px, #20242c ${HOUR_PX}px)` }}
              >
                {drag?.kind === "create" && drag.day === d && (
                  <div style={{ position: "absolute", left: 2, right: 2, top: (Math.min(drag.fromMin, drag.toMin) / 60) * HOUR_PX, height: Math.max((Math.abs(drag.toMin - drag.fromMin) / 60) * HOUR_PX, 2), background: "#3b538866", border: "1px dashed #5a86c7", borderRadius: 3, pointerEvents: "none" }} />
                )}
                {weeklyByDay[d].map(({ i, e }) => {
                  const top = (toMin(e.start) / 60) * HOUR_PX;
                  const h = Math.max(((toMin(e.end) - toMin(e.start)) / 60) * HOUR_PX, 14);
                  return (
                    <div
                      key={`${i}-${d}`}
                      onPointerDown={(ev) => {
                        if (ev.button !== 0) return;
                        ev.stopPropagation();
                        captureGeom();
                        moved.current = false;
                        setDrag({ kind: "move", index: i, grabMin: minAt(ev.clientY) - toMin(e.start), dur: toMin(e.end) - toMin(e.start) });
                      }}
                      onClick={() => { if (moved.current) { moved.current = false; return; } setSel(i); }}
                      title={`${e.name ?? ""} ${e.start}–${e.end} · ${e.value ? "ON" : "OFF"}`}
                      style={{ position: "absolute", top, height: h, left: 2, right: 2, background: sel === i ? "#3b5388" : e.value ? "#2c3a55" : "#3a2c2c", borderLeft: `3px solid ${e.value ? "#5a86c7" : "#a06a6a"}`, borderRadius: 3, padding: "1px 4px", overflow: "hidden", cursor: "grab", color: "#e6e8eb", fontSize: 10, lineHeight: 1.25, touchAction: "none" }}
                    >
                      <div style={{ fontWeight: 600 }}>{e.value ? "ON" : "OFF"}</div>
                      <div style={{ color: "#9ecbff" }}>{e.start}–{e.end}</div>
                      <div onPointerDown={(ev) => { if (ev.button !== 0) return; ev.stopPropagation(); captureGeom(); moved.current = false; setDrag({ kind: "resize", index: i }); }} style={{ position: "absolute", left: 0, right: 0, bottom: 0, height: 6, cursor: "ns-resize" }} />
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* calendar exceptions */}
      {calendarEntries.length > 0 && (
        <div style={{ borderTop: "1px solid #2c313c", maxHeight: 130, overflow: "auto", flexShrink: 0 }}>
          <div style={{ padding: "4px 8px", color: "#8892a0", fontSize: 10, textTransform: "uppercase", letterSpacing: 0.4 }}>Exceptions (override weekly)</div>
          {calendarEntries.map(({ i, e }) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 8px", flexWrap: "wrap" }}>
              <input value={e.name ?? ""} placeholder="name" onChange={(ev) => patchEntry(i, { name: ev.target.value })} style={{ ...inp, width: 130 }} />
              <label style={lbl}>from <input type="date" value={e.from} onChange={(ev) => patchEntry(i, { from: ev.target.value })} style={inp} /></label>
              <label style={lbl}>to <input type="date" value={e.to} onChange={(ev) => patchEntry(i, { to: ev.target.value })} style={inp} /></label>
              <button onClick={() => patchEntry(i, { value: !e.value })} style={toggle(e.value)}>{e.value ? "ON" : "OFF"}</button>
              <button onClick={() => removeEntry(i)} style={{ ...tbBtn, color: "#c98a8a" }}><Trash2 size={12} /></button>
            </div>
          ))}
        </div>
      )}

      {/* weekly entry editor */}
      {sel != null && draft.entries[sel]?.type === "weekly" && (
        <WeeklyEditor
          entry={draft.entries[sel] as WeeklyEntry}
          onChange={(patch) => patchEntry(sel, patch)}
          onDelete={() => removeEntry(sel)}
          onClose={() => setSel(null)}
        />
      )}
    </div>
  );
}

function WeeklyEditor({ entry, onChange, onDelete, onClose }: { entry: WeeklyEntry; onChange: (p: Partial<WeeklyEntry>) => void; onDelete: () => void; onClose: () => void }) {
  const toggleDay = (d: DayKey) => onChange({ days: entry.days.includes(d) ? entry.days.filter((x) => x !== d) : [...entry.days, d] });
  return (
    <div style={{ borderTop: "1px solid #2c313c", padding: "8px 10px", display: "flex", flexWrap: "wrap", alignItems: "center", gap: 10, background: "#15181e", flexShrink: 0 }}>
      <input value={entry.name ?? ""} placeholder="name" onChange={(e) => onChange({ name: e.target.value })} style={{ ...inp, width: 120 }} />
      <div style={{ display: "flex", gap: 3 }}>
        {DAYS.map((d) => (
          <button key={d} onClick={() => toggleDay(d)} style={{ width: 30, padding: "3px 0", fontSize: 10, borderRadius: 3, border: "1px solid #2c313c", cursor: "pointer", color: entry.days.includes(d) ? "#cfe0ff" : "#5a6172", background: entry.days.includes(d) ? "#2c3a55" : "transparent" }}>
            {DAY_LABEL[d]}
          </button>
        ))}
      </div>
      <label style={lbl}>start <input type="time" value={entry.start} onChange={(e) => onChange({ start: e.target.value })} style={inp} /></label>
      <label style={lbl}>end <input type="time" value={entry.end} onChange={(e) => onChange({ end: e.target.value })} style={inp} /></label>
      <label style={lbl}>value <button onClick={() => onChange({ value: !entry.value })} style={toggle(entry.value)}>{entry.value ? "ON" : "OFF"}</button></label>
      <button onClick={onDelete} style={{ ...tbBtn, color: "#c98a8a", marginLeft: "auto" }}><Trash2 size={13} /> Delete</button>
      <button onClick={onClose} style={tbBtn}>Done</button>
    </div>
  );
}

// --- live output readers -----------------------------------------------------
function LiveOut({ comp }: { comp: Component }) {
  const uid = comp.properties["out"]?.uid;
  const v = useValues((s) => (uid != null ? s.values.get(uid) : undefined));
  const on = v === true || v === 1;
  return <span style={{ fontSize: 11, fontWeight: 700, padding: "1px 8px", borderRadius: 3, color: "#fff", background: on ? "#2e7d4f" : "#7a3b3b" }}>{on ? "ON" : "OFF"}</span>;
}
function LiveText({ comp, prop }: { comp: Component; prop: string }) {
  const uid = comp.properties[prop]?.uid;
  const v = useValues((s) => (uid != null ? s.values.get(uid) : undefined));
  return <>{v == null || v === "" ? "—" : String(v)}</>;
}
function LiveTime({ comp, prop }: { comp: Component; prop: string }) {
  const uid = comp.properties[prop]?.uid;
  const v = useValues((s) => (uid != null ? s.values.get(uid) : undefined));
  return <>{fmtEpoch(v)}</>;
}

const muted: React.CSSProperties = { color: "#8892a0", fontSize: 11 };
const tbBtn: React.CSSProperties = { display: "flex", alignItems: "center", gap: 4, padding: "3px 9px", fontSize: 11, color: "#cbd3e0", background: "#2c313c", border: "1px solid #3a4150", borderRadius: 4, cursor: "pointer" };
const tbPrimary: React.CSSProperties = { background: "#2c3a55", borderColor: "#3b5388", color: "#cfe0ff" };
const lbl: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, color: "#8892a0", fontSize: 11 };
const inp: React.CSSProperties = { background: "#0f1115", color: "#e6e8eb", border: "1px solid #2c313c", borderRadius: 3, padding: "2px 5px", fontSize: 12, outline: "none" };
const toggle = (on: boolean): React.CSSProperties => ({ padding: "2px 10px", fontSize: 11, fontWeight: 600, borderRadius: 3, border: `1px solid ${on ? "#3b5388" : "#2c313c"}`, cursor: "pointer", color: on ? "#cfe0ff" : "#8892a0", background: on ? "#2c3a55" : "transparent" });

registerWidget("schedule", SchedulePanel);
