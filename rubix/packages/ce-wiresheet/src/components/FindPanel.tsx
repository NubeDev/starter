import { useEffect, useMemo, useRef, useState } from "react";
import type { Component } from "../lib/engine-types";
import { getRootNodes } from "../lib/rest";

// Command-palette-style component finder. Opens on Cmd/Ctrl+F (or a button),
// searches the WHOLE tree by name / type / path, and on select jumps the view
// to the component's folder and centers + selects it (via the same
// goToComponent path the cross-folder ghosts use).
//
// Fetches the full tree once per open so search spans folders the user isn't
// currently viewing — the whole point on a big flow.

interface Hit {
  uid: number;
  name: string;
  type: string;
  path: string; // stripped of leading "root/"
  parent: number;
  here: boolean; // in the folder currently being viewed
}

export function FindPanel({
  open,
  currentParentUid,
  onClose,
  onPick,
}: {
  open: boolean;
  currentParentUid: number;
  onClose: () => void;
  onPick: (uid: number) => void;
}) {
  const [query, setQuery] = useState("");
  const [all, setAll] = useState<Hit[] | null>(null);
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Load the full tree each time the panel opens (cheap on the scale we run at;
  // keeps results fresh after edits).
  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSel(0);
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        const flat: Hit[] = [];
        const walk = (c: Component) => {
          // Skip root itself; it's not a navigable target.
          if (c.uid !== 0) {
            const path = c.path.startsWith("root/") ? c.path.slice(5) : c.path;
            flat.push({
              uid: c.uid,
              name: c.name || c.type,
              type: c.type,
              path,
              parent: c.parent,
              here: c.parent === currentParentUid,
            });
          }
          c.children?.forEach(walk);
        };
        resp.nodes.forEach(walk);
        setAll(flat);
      } catch {
        if (!cancelled) setAll([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, currentParentUid]);

  // Focus the input when opened.
  useEffect(() => {
    if (open) {
      // Defer so the element exists.
      const t = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(t);
    }
  }, [open]);

  const results = useMemo(() => {
    if (!all) return [];
    const q = query.trim().toLowerCase();
    if (!q) return all.slice(0, 50);
    const scored = all
      .map((h) => {
        const name = h.name.toLowerCase();
        // Rank: exact name > name prefix > name contains > path/type contains.
        let score = -1;
        if (name === q) score = 0;
        else if (name.startsWith(q)) score = 1;
        else if (name.includes(q)) score = 2;
        else if (h.path.toLowerCase().includes(q) || h.type.toLowerCase().includes(q)) score = 3;
        return { h, score };
      })
      .filter((x) => x.score >= 0)
      // Current-folder hits first, then by match score, then name. So "what's
      // on this level" floats to the top and is also badged in the row.
      .sort(
        (a, b) =>
          Number(b.h.here) - Number(a.h.here) ||
          a.score - b.score ||
          a.h.name.localeCompare(b.h.name),
      )
      .slice(0, 50)
      .map((x) => x.h);
    return scored;
  }, [all, query]);

  // Keep the selected row in view + clamp selection when results change.
  useEffect(() => {
    if (sel >= results.length) setSel(0);
  }, [results, sel]);
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-idx="${sel}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  if (!open) return null;

  // (FindHeader defined at module scope below.)

  const pick = (h: Hit | undefined) => {
    if (!h) return;
    onPick(h.uid);
    onClose();
  };

  return (
    <div
      // Backdrop — click outside closes.
      onMouseDown={onClose}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 200,
        background: "rgba(0,0,0,0.35)",
        display: "flex",
        justifyContent: "center",
        alignItems: "flex-start",
        paddingTop: "12vh",
      }}
    >
      <div
        onMouseDown={(e) => e.stopPropagation()}
        style={{
          width: 480,
          maxWidth: "90vw",
          background: "#1a1d24",
          border: "1px solid #2c313c",
          borderRadius: 8,
          boxShadow: "0 12px 40px rgba(0,0,0,0.6)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          fontFamily: "-apple-system, system-ui, sans-serif",
        }}
      >
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSel(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            } else if (e.key === "ArrowDown") {
              e.preventDefault();
              setSel((s) => Math.min(results.length - 1, s + 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSel((s) => Math.max(0, s - 1));
            } else if (e.key === "Enter") {
              e.preventDefault();
              pick(results[sel]);
            }
            e.stopPropagation();
          }}
          placeholder="Find component by name, type, or path…"
          spellCheck={false}
          style={{
            background: "#0f1115",
            color: "#e6e8eb",
            border: "none",
            borderBottom: "1px solid #2c313c",
            padding: "12px 14px",
            fontSize: 14,
            fontFamily: "ui-monospace, SFMono-Regular, monospace",
            outline: "none",
          }}
        />
        <div ref={listRef} style={{ maxHeight: "50vh", overflowY: "auto" }}>
          {all == null ? (
            <div style={{ padding: "12px 14px", color: "#5a6172", fontSize: 12 }}>loading…</div>
          ) : results.length === 0 ? (
            <div style={{ padding: "12px 14px", color: "#5a6172", fontSize: 12 }}>
              no matches
            </div>
          ) : (
            results.map((h, i) => {
              // Section dividers: "this folder" above the first here-hit,
              // "elsewhere" at the here→other boundary.
              const prev = i > 0 ? results[i - 1] : null;
              const showHereHeader = h.here && (prev === null || !prev.here);
              const showElsewhereHeader = !h.here && (prev === null || prev.here);
              return (
                <div key={h.uid}>
                  {showHereHeader && <FindHeader label="this folder" />}
                  {showElsewhereHeader && <FindHeader label="elsewhere" />}
                  <button
                    data-idx={i}
                    onMouseEnter={() => setSel(i)}
                    onClick={() => pick(h)}
                    style={{
                      display: "flex",
                      width: "100%",
                      textAlign: "left",
                      alignItems: "baseline",
                      gap: 8,
                      padding: "8px 14px 8px 12px",
                      background: i === sel ? "#2c3a55" : "transparent",
                      border: "none",
                      // Left accent on same-folder rows so they read as "here"
                      // even mid-scroll, past the section header.
                      borderLeft: `2px solid ${h.here ? "#4a9eff" : "transparent"}`,
                      cursor: "pointer",
                      fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    }}
                  >
                    <span style={{ color: "#e6e8eb", fontSize: 13, flexShrink: 0 }}>{h.name}</span>
                    <span
                      style={{
                        color: "#5a6172",
                        fontSize: 11,
                        flex: 1,
                        minWidth: 0,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      title={`${h.path} · ${h.type}`}
                    >
                      {/* For same-folder hits the path is the current view, so
                          show the type instead of repeating the folder. */}
                      {h.here ? h.type : h.path}
                    </span>
                    {!h.here && (
                      <span style={{ color: "#8892a0", fontSize: 10, flexShrink: 0 }}>{h.type}</span>
                    )}
                  </button>
                </div>
              );
            })
          )}
        </div>
        <div
          style={{
            padding: "6px 14px",
            borderTop: "1px solid #2c313c",
            color: "#5a6172",
            fontSize: 10,
            fontFamily: "ui-monospace, monospace",
          }}
        >
          ↑↓ navigate · ↵ go · esc close
        </div>
      </div>
    </div>
  );
}

// Section header inside the results list ("this folder" / "elsewhere").
function FindHeader({ label }: { label: string }) {
  return (
    <div
      style={{
        padding: "6px 14px 3px 14px",
        color: "#5a6172",
        fontSize: 9,
        textTransform: "uppercase",
        letterSpacing: 0.5,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      {label}
    </div>
  );
}
