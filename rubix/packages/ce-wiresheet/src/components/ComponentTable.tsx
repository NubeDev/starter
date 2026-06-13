import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues } from "../lib/store";
import { ROLE_NORMAL, ROLE_STATUS, CATEGORY_INPUT, CATEGORY_OUTPUT } from "../lib/engine-types";
import type { Component } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import { facetFor, rawFacet, aliasLabel, type PropFacet, type ComponentFacet } from "../lib/facet";
import { Layers, ChevronRight, ArrowUp, ArrowDown } from "lucide-react";

// Flat list view of the CURRENT folder's components. Each component is one
// self-labelling line — name followed by `<label> <value>` pairs (the inline
// name resolves to the facet label, fallback to the raw prop name). No column
// headers, no per-type grids: each row carries its own props, so a folder of 100
// mixed components reads cleanly. Live values, search, name sort; a selected
// component the search filters out is pinned at the bottom.

const fmtCell = (v: DecodedValue | undefined, facet: PropFacet | undefined): string => {
  if (v === undefined || v === null) return "—";
  const al = aliasLabel(facet?.aliases, v);
  if (al) return al;
  let s: string;
  if (typeof v === "number" && facet?.decimals != null) s = v.toFixed(facet.decimals);
  else s = String(v);
  return facet?.unit ? `${s} ${facet.unit}` : s;
};

const catRank = (c: number) => (c === CATEGORY_INPUT ? 0 : c === CATEGORY_OUTPUT ? 1 : 2);

interface PropCell {
  uid: number;
  label: string;
  category: number;
}

export function ComponentTable({
  currentParentUid,
  selectedUids,
  onSelectRow,
  onDrillIn,
  onRowsChange,
}: {
  currentParentUid: number;
  selectedUids: number[];
  onSelectRow: (uid: number, additive: boolean) => void;
  onDrillIn: (uid: number) => void;
  onRowsChange: (uids: number[]) => void;
}) {
  const [showHidden, setShowHidden] = useState(false);
  const [query, setQuery] = useState("");
  const [dir, setDir] = useState<1 | -1>(1);
  const scrollRef = useRef<HTMLDivElement>(null);

  const components = useStructural((s) => s.components);
  const allRows = useMemo(
    () => Array.from(components.values()).filter((c) => c.parent === currentParentUid),
    [components, currentParentUid],
  );

  const facets = useMemo(
    () =>
      new Map<number, ComponentFacet>(allRows.map((c) => [c.uid, facetFor(c.uid, rawFacet(c.properties))])),
    [allRows],
  );

  // The visible prop cells for a component: user-facing (+ not hidden unless the
  // toggle is on), ordered input→output→config then by facet order then name.
  const cellsFor = (c: Component): PropCell[] => {
    const facet = facets.get(c.uid);
    const out: (PropCell & { name: string })[] = [];
    for (const [name, p] of Object.entries(c.properties)) {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      const f = facet?.get(p.uid);
      if (!showHidden && f?.hidden) continue;
      out.push({ uid: p.uid, label: f?.label || name, category: p.category, name });
    }
    out.sort(
      (a, b) =>
        catRank(a.category) - catRank(b.category) ||
        (facets.get(c.uid)?.get(a.uid)?.order ?? 1e9) - (facets.get(c.uid)?.get(b.uid)?.order ?? 1e9) ||
        a.name.localeCompare(b.name),
    );
    return out;
  };

  const q = query.trim().toLowerCase();
  const matches = (c: Component) =>
    !q || (c.name || c.type).toLowerCase().includes(q) || c.type.toLowerCase().includes(q);

  const rows = useMemo(
    () =>
      allRows
        .filter(matches)
        .sort((a, b) => (a.name || a.type).localeCompare(b.name || b.type) * dir),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allRows, q, dir],
  );

  // Live values for every cell + status, in one shallow read.
  const watchUids = useMemo(() => {
    const set = new Set<number>();
    for (const c of allRows)
      for (const p of Object.values(c.properties))
        if ((p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL || p.systemRole === ROLE_STATUS) set.add(p.uid);
    return [...set];
  }, [allRows]);
  const values = useValues(
    useShallow((s) => {
      const out: Record<number, DecodedValue | undefined> = {};
      for (const uid of watchUids) out[uid] = s.values.get(uid);
      return out;
    }),
  );

  useEffect(() => {
    onRowsChange(allRows.map((c) => c.uid));
    return () => onRowsChange([]);
  }, [allRows, onRowsChange]);

  const firstSel = selectedUids[0];
  useEffect(() => {
    if (firstSel == null) return;
    scrollRef.current
      ?.querySelector<HTMLElement>(`[data-uid="${firstSel}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [firstSel, rows]);

  const sel = new Set(selectedUids);
  // Selected components the search filtered out — pinned at the bottom.
  const orphans = q ? allRows.filter((c) => sel.has(c.uid) && !matches(c)) : [];

  const renderRow = (c: Component) => {
    const isFolder = (c.childrenCount ?? 0) > 0;
    return (
      <div
        key={c.uid}
        data-uid={c.uid}
        onClick={(e) => onSelectRow(c.uid, e.shiftKey || e.metaKey || e.ctrlKey)}
        onDoubleClick={() => isFolder && onDrillIn(c.uid)}
        style={{
          display: "flex",
          alignItems: "baseline",
          flexWrap: "wrap",
          gap: "2px 14px",
          padding: "5px 10px",
          cursor: "pointer",
          background: sel.has(c.uid) ? "#2c3a55" : "transparent",
          borderBottom: "1px solid #1f232b",
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
        }}
      >
        <span style={{ display: "flex", alignItems: "center", gap: 4, minWidth: 120, fontWeight: 600 }}>
          {isFolder && <Layers size={12} color="#9ecbff" />}
          <span style={{ color: "#e6e8eb" }}>{c.name || c.type}</span>
          {isFolder && <ChevronRight size={12} color="#5a6172" />}
        </span>
        {cellsFor(c).map((p) => (
          <span key={p.uid} style={{ display: "inline-flex", gap: 5, alignItems: "baseline" }}>
            <span style={{ color: "#5a6172" }}>{p.label}</span>
            <span style={{ color: p.category === CATEGORY_INPUT ? "#cbd3e0" : "#e6e8eb" }}>
              {fmtCell(values[p.uid], facets.get(c.uid)?.get(p.uid))}
            </span>
          </span>
        ))}
      </div>
    );
  };

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "#15181e",
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        fontSize: 12,
        minWidth: 0,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 10px",
          borderBottom: "1px solid #2c313c",
          flexShrink: 0,
        }}
      >
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="search components…"
          spellCheck={false}
          style={{
            flex: 1,
            minWidth: 0,
            background: "#222731",
            color: "#cbd3e0",
            border: "1px solid #2c313c",
            borderRadius: 3,
            padding: "4px 7px",
            fontSize: 11,
            fontFamily: "ui-monospace, monospace",
            outline: "none",
          }}
        />
        <button
          onClick={() => setDir((d) => (d === 1 ? -1 : 1))}
          title={`Sort by name ${dir === 1 ? "ascending" : "descending"}`}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 3,
            background: "transparent",
            border: "1px solid #2c313c",
            borderRadius: 3,
            color: "#8892a0",
            cursor: "pointer",
            padding: "3px 6px",
            fontSize: 11,
            flexShrink: 0,
          }}
        >
          name {dir === 1 ? <ArrowUp size={11} /> : <ArrowDown size={11} />}
        </button>
        <label style={{ display: "flex", alignItems: "center", gap: 4, color: "#8892a0", flexShrink: 0 }}>
          <input type="checkbox" checked={showHidden} onChange={(e) => setShowHidden(e.target.checked)} />
          hidden
        </label>
      </div>

      <div ref={scrollRef} style={{ overflow: "auto", flex: 1 }}>
        {rows.length === 0 && orphans.length === 0 ? (
          <div style={{ padding: 12, color: "#5a6172" }}>
            {allRows.length === 0 ? "no components in this folder" : "no matches"}
          </div>
        ) : (
          rows.map(renderRow)
        )}

        {orphans.length > 0 && (
          <>
            <div
              style={{
                padding: "5px 10px",
                background: "#221a1a",
                borderTop: "1px solid #3a2a2a",
                borderBottom: "1px solid #3a2a2a",
                color: "#c9a86a",
                fontSize: 10,
                textTransform: "uppercase",
                letterSpacing: 0.4,
              }}
            >
              selected · filtered out ({orphans.length})
            </div>
            {orphans.map(renderRow)}
          </>
        )}
      </div>
    </div>
  );
}
