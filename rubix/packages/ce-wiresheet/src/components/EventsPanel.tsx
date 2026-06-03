import { useEffect, useRef, useState } from "react";
import {
  clearEvents,
  events,
  eventsVersion,
  type EventKind,
  type WireEvent,
} from "../lib/instrumentation";

// Live event log. Mirrors what the wire layer sees: WS open/close, schema
// arrival, subscribe/unsubscribe sends, binary frames (with value+status uid
// counts), topology pushes, REST mutations. Polls a module-level ring buffer
// via rAF and rewrites a single <pre> innerText — never re-renders React.
//
// Toggle button is rendered inline so it can sit alongside the PerfPanel
// collapse button up in the corner; the actual list is a separate fixed
// overlay so its height/scroll doesn't fight the metrics panel.

const COLLAPSED_KEY = "ce-ui.eventspanel.collapsed";

const KIND_COLOR: Record<EventKind, string> = {
  "ws-open": "#4ade80",
  "ws-close": "#ef4444",
  schema: "#9ecbff",
  subscribe: "#9ecbff",
  unsubscribe: "#8892a0",
  frame: "#a8b3c7",
  topology: "#ffd166",
  rest: "#f59e0b",
};

const KIND_LABEL: Record<EventKind, string> = {
  "ws-open": "OPEN",
  "ws-close": "CLOSE",
  schema: "SCHEMA",
  subscribe: "SUB",
  unsubscribe: "UNSUB",
  frame: "FRAME",
  topology: "TOPO",
  rest: "REST",
};

const ALL_KINDS: EventKind[] = [
  "ws-open",
  "ws-close",
  "schema",
  "subscribe",
  "unsubscribe",
  "frame",
  "topology",
  "rest",
];

export function EventsPanel() {
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    try {
      return window.localStorage.getItem(COLLAPSED_KEY) !== "0";
    } catch {
      return true;
    }
  });
  const [paused, setPaused] = useState(false);
  // Excluded kinds. Frames dominate the log by volume when subscribed, so
  // they're off by default — user can flip them on to see every binary push.
  const [excluded, setExcluded] = useState<Set<EventKind>>(() => new Set(["frame"]));

  useEffect(() => {
    try {
      window.localStorage.setItem(COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
      /* private mode etc */
    }
  }, [collapsed]);

  const listRef = useRef<HTMLDivElement>(null);
  const lastVersion = useRef(-1);
  const stickToBottom = useRef(true);

  useEffect(() => {
    if (collapsed) return;
    let raf = 0;
    const tick = () => {
      raf = requestAnimationFrame(tick);
      if (paused) return;
      if (eventsVersion.v === lastVersion.current) return;
      lastVersion.current = eventsVersion.v;
      renderList(listRef.current, events, excluded, stickToBottom.current);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [collapsed, paused, excluded]);

  // Track scroll position so a user scrolling back to inspect history doesn't
  // get yanked to the bottom on every new event. Auto-resume sticky-bottom
  // once they scroll back down to the last few rows.
  const onScroll = () => {
    const el = listRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 12;
    stickToBottom.current = atBottom;
  };

  if (collapsed) {
    return (
      <button
        onClick={() => setCollapsed(false)}
        title="Show events log"
        style={{
          position: "fixed",
          top: 12,
          right: 304,
          zIndex: 30,
          padding: "6px 10px",
          background: "rgba(20, 23, 30, 0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          color: "#cbd3e0",
          fontSize: 11,
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
          cursor: "pointer",
        }}
      >
        events ▾
      </button>
    );
  }

  return (
    <div
      style={{
        position: "fixed",
        top: 12,
        right: 304,
        bottom: 12,
        width: 420,
        zIndex: 30,
        background: "rgba(20, 23, 30, 0.96)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        color: "#e6e8eb",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid #2c313c",
        }}
      >
        <span style={{ fontWeight: 600, fontSize: 12, flex: 1 }}>events</span>
        <button
          onClick={() => setPaused((p) => !p)}
          title={paused ? "Resume" : "Pause"}
          style={btn(paused ? "#3b6eff" : "transparent")}
        >
          {paused ? "▶ resume" : "❚❚ pause"}
        </button>
        <button onClick={() => clearEvents()} title="Clear log" style={btn("transparent")}>
          clear
        </button>
        <button
          onClick={() => setCollapsed(true)}
          title="Hide events log"
          style={btn("transparent")}
        >
          ▴
        </button>
      </div>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 4,
          padding: "6px 10px",
          borderBottom: "1px solid #2c313c",
        }}
      >
        {ALL_KINDS.map((k) => {
          const on = !excluded.has(k);
          return (
            <button
              key={k}
              onClick={() =>
                setExcluded((cur) => {
                  const next = new Set(cur);
                  if (next.has(k)) next.delete(k);
                  else next.add(k);
                  return next;
                })
              }
              style={{
                fontSize: 10,
                padding: "2px 6px",
                background: on ? KIND_COLOR[k] : "transparent",
                color: on ? "#0f1115" : KIND_COLOR[k],
                border: `1px solid ${KIND_COLOR[k]}`,
                borderRadius: 2,
                cursor: "pointer",
                fontFamily: "inherit",
                opacity: on ? 1 : 0.55,
              }}
            >
              {KIND_LABEL[k]}
            </button>
          );
        })}
      </div>
      <div
        ref={listRef}
        onScroll={onScroll}
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "4px 8px",
          lineHeight: 1.45,
          whiteSpace: "pre",
        }}
      />
    </div>
  );
}

function btn(bg: string): React.CSSProperties {
  return {
    fontSize: 11,
    padding: "2px 8px",
    background: bg,
    color: "#cbd3e0",
    border: "1px solid #2c313c",
    borderRadius: 3,
    cursor: "pointer",
    fontFamily: "inherit",
  };
}

// Imperative render — avoids React reconciliation on every event tick.
// Builds a flat HTML string with per-line color spans and writes innerHTML
// once. Cheap because event lines are short and the buffer is capped at 500.
function renderList(
  el: HTMLDivElement | null,
  buf: WireEvent[],
  excluded: Set<EventKind>,
  stickToBottom: boolean,
) {
  if (!el) return;
  const t0 = buf.length > 0 ? buf[0].t : 0;
  let html = "";
  for (let i = 0; i < buf.length; i++) {
    const e = buf[i];
    if (excluded.has(e.kind)) continue;
    const rel = ((e.t - t0) / 1000).toFixed(2).padStart(7);
    const color = KIND_COLOR[e.kind];
    const label = KIND_LABEL[e.kind].padEnd(6);
    html +=
      `<div><span style="color:#5a6172">${rel}s</span> ` +
      `<span style="color:${color}">${label}</span> ` +
      `<span style="color:#cbd3e0">${escapeHtml(e.text)}</span></div>`;
  }
  el.innerHTML = html;
  if (stickToBottom) el.scrollTop = el.scrollHeight;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
