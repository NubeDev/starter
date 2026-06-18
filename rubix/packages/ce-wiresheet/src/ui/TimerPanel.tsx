// The `timer` widget — start a manual timer and show when it ends. Duration can
// be set as seconds OR as a target date/time (we compute the seconds from now
// until then). Binds to the selected timer component: reads `out` / `timerEnd`
// outputs live, Start/Cancel call the `startTimer` / `cancelTimer` actions.

import { useEffect, useState } from "react";
import { useStructural, useValues } from "../lib/store";
import { Play, Square } from "lucide-react";
import { fmtDateTime } from "../lib/format";
import { registerWidget, type WidgetProps } from "./registry";

const PRESETS: { label: string; sec: number }[] = [
  { label: "5m", sec: 300 },
  { label: "30m", sec: 1800 },
  { label: "1h", sec: 3600 },
  { label: "2h", sec: 7200 },
];

function fmtDur(s: number): string {
  if (s <= 0) return "0s";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return [h ? `${h}h` : "", m ? `${m}m` : "", sec || (!h && !m) ? `${sec}s` : ""].filter(Boolean).join(" ");
}

function TimerPanel({ ctx }: WidgetProps) {
  const comp = useStructural((s) => (ctx.componentUid != null ? s.components.get(ctx.componentUid) : undefined));
  const outUid = comp?.properties["out"]?.uid;
  const endUid = comp?.properties["timerEnd"]?.uid;
  const out = useValues((s) => (outUid != null ? s.values.get(outUid) : undefined));
  const timerEnd = useValues((s) => (endUid != null ? s.values.get(endUid) : undefined));
  const running = out === true || out === 1;
  const endEpoch = typeof timerEnd === "number" ? timerEnd : 0;
  const endMs = endEpoch > 0 ? (Math.abs(endEpoch) < 1e12 ? endEpoch * 1000 : endEpoch) : 0;

  const [mode, setMode] = useState<"seconds" | "until">("seconds");
  const [sec, setSec] = useState(600);
  const [until, setUntil] = useState("");

  // tick once a second to drive the live countdown
  const [nowMs, setNowMs] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const untilSec = until ? Math.round((new Date(until).getTime() - nowMs) / 1000) : 0;
  const duration = mode === "seconds" ? Math.max(0, Math.floor(sec)) : Math.max(0, untilSec);
  const canCall = ctx.componentUid != null && !!ctx.callAction;
  const remaining = running && endMs > 0 ? Math.max(0, Math.round((endMs - nowMs) / 1000)) : 0;

  // manifest: startTimer(seconds: uint). (`duration` is also a settable input
  // property, but the action takes its own `seconds` param.)
  const start = () => canCall && duration > 0 && ctx.callAction!(ctx.componentUid!, "startTimer", { seconds: duration });
  const cancel = () => canCall && ctx.callAction!(ctx.componentUid!, "cancelTimer", {});

  if (!comp) return null;

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 12, fontSize: 12, color: "#e6e8eb", maxWidth: 360 }}>
      {/* live state */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ fontWeight: 700, padding: "2px 10px", borderRadius: 3, color: "#fff", background: running ? "#2e7d4f" : "#3a4150" }}>
          {running ? "RUNNING" : "IDLE"}
        </span>
        {running && (
          <span style={{ color: "#8892a0" }}>
            ends <b style={{ color: "#e6e8eb" }}>{fmtDateTime(endEpoch, "datetime")}</b> · <b style={{ color: "#9ecbff" }}>{fmtDur(remaining)}</b> left
          </span>
        )}
      </div>

      {/* duration input */}
      <div style={{ display: "flex", gap: 4 }}>
        {(["seconds", "until"] as const).map((m) => (
          <button key={m} onClick={() => setMode(m)} style={seg(mode === m)}>
            {m === "seconds" ? "Duration" : "Until date/time"}
          </button>
        ))}
      </div>

      {mode === "seconds" ? (
        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          <input type="number" min={0} value={sec} onChange={(e) => setSec(Number(e.target.value))} style={{ ...inp, width: 90 }} />
          <span style={{ color: "#8892a0" }}>seconds</span>
          {PRESETS.map((p) => (
            <button key={p.label} onClick={() => setSec(p.sec)} style={chip}>{p.label}</button>
          ))}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <input type="datetime-local" value={until} onChange={(e) => setUntil(e.target.value)} style={{ ...inp, width: "100%" }} />
          <span style={{ color: untilSec > 0 ? "#8892a0" : "#e0707a", fontSize: 11 }}>
            {until ? (untilSec > 0 ? `= ${fmtDur(untilSec)} (${duration}s) from now` : "that time is in the past") : "pick a local date & time"}
          </span>
        </div>
      )}

      <div style={{ display: "flex", gap: 6 }}>
        <button onClick={start} disabled={!canCall || duration <= 0} style={{ ...btn, ...btnPrimary, opacity: !canCall || duration <= 0 ? 0.4 : 1 }}>
          <Play size={13} /> Start
        </button>
        <button onClick={cancel} disabled={!canCall || !running} style={{ ...btn, opacity: !canCall || !running ? 0.4 : 1 }}>
          <Square size={13} /> Cancel
        </button>
      </div>
    </div>
  );
}

const inp: React.CSSProperties = { background: "#0f1115", color: "#e6e8eb", border: "1px solid #2c313c", borderRadius: 3, padding: "3px 6px", fontSize: 12, outline: "none" };
const btn: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, padding: "5px 12px", fontSize: 12, color: "#cbd3e0", background: "#2c313c", border: "1px solid #3a4150", borderRadius: 4, cursor: "pointer" };
const btnPrimary: React.CSSProperties = { background: "#2c3a55", borderColor: "#3b5388", color: "#cfe0ff" };
const seg = (on: boolean): React.CSSProperties => ({ padding: "4px 12px", fontSize: 11, borderRadius: 4, border: `1px solid ${on ? "#3b5388" : "#2c313c"}`, cursor: "pointer", color: on ? "#cfe0ff" : "#8892a0", background: on ? "#2c3a55" : "transparent" });
const chip: React.CSSProperties = { padding: "2px 8px", fontSize: 11, borderRadius: 3, border: "1px solid #2c313c", cursor: "pointer", color: "#9aa3b2", background: "transparent" };

registerWidget("timer", TimerPanel);
