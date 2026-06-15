import { useEffect, useRef } from "react";
import { metrics } from "../lib/instrumentation";

// Bottom-bar connection indicator + handle for the diagnostics/events drawer.
// The dot + label are driven by a self-contained rAF reading `metrics.wsConnected`
// (set directly by the WS layer), so it stays live regardless of whether the
// drawer — and the DiagPanel inside it — is mounted.
export function ConnectionStatus({
  open,
  onToggle,
}: {
  open: boolean;
  onToggle: () => void;
}) {
  const rDot = useRef<HTMLSpanElement>(null);
  const rTxt = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    let raf = 0;
    let last: boolean | null = null;
    const tick = () => {
      raf = requestAnimationFrame(tick);
      if (last === metrics.wsConnected) return;
      last = metrics.wsConnected;
      if (rDot.current) rDot.current.style.background = last ? "#4ade80" : "#ef4444";
      if (rTxt.current) rTxt.current.textContent = last ? "connected" : "disconnected";
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  return (
    <button
      onClick={onToggle}
      title={open ? "Hide diagnostics & events" : "Show diagnostics & events"}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 7,
        padding: "3px 10px",
        background: open ? "#222731" : "transparent",
        border: "1px solid #2c313c",
        borderRadius: 5,
        color: "#cbd3e0",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        cursor: "pointer",
      }}
    >
      <span
        ref={rDot}
        style={{ width: 8, height: 8, borderRadius: 4, background: "#ef4444", display: "inline-block" }}
      />
      <span ref={rTxt}>disconnected</span>
      <span style={{ color: "#5a6172", fontSize: 10 }}>{open ? "▾" : "▴"}</span>
    </button>
  );
}
