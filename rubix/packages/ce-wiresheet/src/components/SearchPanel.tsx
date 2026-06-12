import { useEffect, useMemo, useRef, useState } from "react";
import type { Component } from "../lib/engine-types";
import { ROLE_NORMAL } from "../lib/engine-types";
import { getRootNodes } from "../lib/rest";
import { parseFacet, rawFacet } from "../lib/facet";

// Docked, collapsible search panel (top-left). Unlike the Cmd/Ctrl+F command
// palette (FindPanel), this stays out of the way and searches BOTH component
// names AND per-prop presentation text — facet labels and value aliases — so
// you can find a row by the human label/alias, not just the component name.
// Selecting a hit navigates the view to that component (same goToComponent path
// as the cross-folder ghosts).
//
// The whole tree is fetched (with properties + facets) each time the panel is
// expanded, so results span folders you're not currently viewing and stay fresh
// after edits. Cheap at the scale we run at.

interface Hit {
  compUid: number; // navigation target
  compName: string;
  type: string;
  path: string; // stripped of leading "root/"
  here: boolean; // in the folder currently being viewed
  // Prop-level fields (absent on component hits).
  propName?: string;
  label?: string; // facet label
  aliasText?: string; // space-joined alias labels
}

export function SearchPanel({
  currentParentUid,
  onPick,
}: {
  currentParentUid: number;
  onPick: (uid: number) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [all, setAll] = useState<Hit[] | null>(null);
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // (Re)load the full tree whenever the panel is expanded.
  useEffect(() => {
    if (!open) return;
    setSel(0);
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        const flat: Hit[] = [];
        const walk = (c: Component) => {
          if (c.uid !== 0) {
            const path = c.path.startsWith("root/") ? c.path.slice(5) : c.path;
            const here = c.parent === currentParentUid;
            const compName = c.name || c.type;
            flat.push({ compUid: c.uid, compName, type: c.type, path, here });
            // Index each user-prop's presentation text (label + aliases). Only
            // emit a prop row when it actually carries a label or aliases — a
            // bare prop name is already visible under its component.
            const facet = parseFacet(rawFacet(c.properties) ?? "");
            for (const [propName, p] of Object.entries(c.properties)) {
              if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
              const f = facet.get(p.uid);
              const aliasText = f?.aliases?.map((a) => a.label).join(" ") ?? "";
              if (!f?.label && !aliasText) continue;
              flat.push({
                compUid: c.uid,
                compName,
                type: c.type,
                path,
                here,
                propName,
                label: f?.label,
                aliasText,
              });
            }
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

  useEffect(() => {
    if (open) {
      const t = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(t);
    }
  }, [open]);

  const results = useMemo(() => {
    if (!all) return [];
    const q = query.trim().toLowerCase();
    if (!q) return all.filter((h) => !h.propName).slice(0, 60);
    const scored = all
      .map((h) => {
        let score = -1;
        if (h.propName) {
          const label = (h.label ?? "").toLowerCase();
          const al = (h.aliasText ?? "").toLowerCase();
          const pn = h.propName.toLowerCase();
          if (label === q || al.split(" ").includes(q)) score = 1;
          else if (label.startsWith(q) || pn.startsWith(q)) score = 2;
          else if (label.includes(q) || al.includes(q) || pn.includes(q)) score = 3;
        } else {
          const name = h.compName.toLowerCase();
          if (name === q) score = 0;
          else if (name.startsWith(q)) score = 1;
          else if (name.includes(q)) score = 2;
          else if (h.path.toLowerCase().includes(q) || h.type.toLowerCase().includes(q)) score = 3;
        }
        return { h, score };
      })
      .filter((x) => x.score >= 0)
      .sort(
        (a, b) =>
          Number(b.h.here) - Number(a.h.here) ||
          a.score - b.score ||
          a.h.compName.localeCompare(b.h.compName),
      )
      .slice(0, 80)
      .map((x) => x.h);
    return scored;
  }, [all, query]);

  useEffect(() => {
    if (sel >= results.length) setSel(0);
  }, [results, sel]);
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-idx="${sel}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  const pick = (h: Hit | undefined) => {
    if (!h) return;
    onPick(h.compUid);
  };

  return (
    <div
      onPointerDown={(e) => e.stopPropagation()}
      style={{
        position: "fixed",
        top: 12,
        left: 12,
        zIndex: 32,
        width: open ? 300 : "auto",
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 6,
        boxShadow: "0 6px 20px rgba(0,0,0,0.5)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        fontFamily: "-apple-system, system-ui, sans-serif",
        maxHeight: open ? "70vh" : undefined,
      }}
    >
      <button
        onClick={() => setOpen((v) => !v)}
        title={open ? "Collapse search" : "Search components & props"}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "6px 10px",
          background: "transparent",
          border: "none",
          color: "#c7ccd6",
          cursor: "pointer",
          fontSize: 12,
          fontWeight: 600,
        }}
      >
        <span style={{ color: "#5a6172", fontSize: 10 }}>{open ? "▾" : "▸"}</span>
        <span>Search</span>
        {!open && <span style={{ color: "#5a6172", fontWeight: 400, fontSize: 11 }}>⌕</span>}
      </button>

      {open && (
        <>
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
                setOpen(false);
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
            placeholder="name, label, or alias…"
            spellCheck={false}
            style={{
              background: "#0f1115",
              color: "#e6e8eb",
              border: "none",
              borderTop: "1px solid #2c313c",
              borderBottom: "1px solid #2c313c",
              padding: "8px 10px",
              fontSize: 13,
              fontFamily: "ui-monospace, SFMono-Regular, monospace",
              outline: "none",
            }}
          />
          <div ref={listRef} style={{ overflowY: "auto" }}>
            {all == null ? (
              <div style={{ padding: "10px", color: "#5a6172", fontSize: 12 }}>loading…</div>
            ) : results.length === 0 ? (
              <div style={{ padding: "10px", color: "#5a6172", fontSize: 12 }}>no matches</div>
            ) : (
              results.map((h, i) => (
                <button
                  key={`${h.compUid}:${h.propName ?? ""}:${i}`}
                  data-idx={i}
                  onMouseEnter={() => setSel(i)}
                  onClick={() => pick(h)}
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "stretch",
                    gap: 2,
                    width: "100%",
                    textAlign: "left",
                    padding: "6px 10px",
                    background: i === sel ? "#2c3a55" : "transparent",
                    border: "none",
                    borderLeft: `2px solid ${h.here ? "#4a9eff" : "transparent"}`,
                    cursor: "pointer",
                    fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                    {h.propName ? (
                      <>
                        <span style={{ color: "#e6e8eb", fontSize: 12 }}>
                          {h.label || h.propName}
                        </span>
                        <span
                          style={{
                            color: "#7a8aa0",
                            fontSize: 9,
                            border: "1px solid #2c3a55",
                            borderRadius: 3,
                            padding: "0 4px",
                            flexShrink: 0,
                          }}
                        >
                          prop
                        </span>
                        <span
                          style={{
                            color: "#5a6172",
                            fontSize: 11,
                            marginLeft: "auto",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                          title={h.path}
                        >
                          {h.compName}
                        </span>
                      </>
                    ) : (
                      <>
                        <span style={{ color: "#e6e8eb", fontSize: 12 }}>{h.compName}</span>
                        <span
                          style={{
                            color: "#5a6172",
                            fontSize: 11,
                            marginLeft: "auto",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                          title={`${h.path} · ${h.type}`}
                        >
                          {h.here ? h.type : h.path}
                        </span>
                      </>
                    )}
                  </div>
                  {h.propName && h.aliasText && (
                    <div
                      style={{
                        color: "#5a6172",
                        fontSize: 10,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {h.aliasText}
                    </div>
                  )}
                </button>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );
}
