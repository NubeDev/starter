import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues } from "../lib/store";
import {
  ROLE_NORMAL,
  ROLE_STATUS,
  CATEGORY_INPUT,
  CATEGORY_OUTPUT,
  CATEGORY_CONFIG,
} from "../lib/engine-types";
import type { Component } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import { facetFor, rawFacet, aliasLabel, type PropFacet, type ComponentFacet } from "../lib/facet";
import { Layers, ChevronRight, ArrowUp, ArrowDown } from "lucide-react";

// Table view of the CURRENT folder's components. Each component is a row; its
// props are aligned into per-slot columns grouped under Inputs / Outputs / Config
// category headers (no per-prop headers — each cell self-labels with the facet
// label, fallback raw prop name). Aligning by slot lets you scan a column even
// across mixed types. Live values, search, name sort; a selected component the
// search filters out is pinned at the bottom.

const fmtCell = (v: DecodedValue | undefined, facet: PropFacet | undefined): string => {
  if (v === undefined || v === null) return "—";
  const al = aliasLabel(facet?.aliases, v);
  if (al) return al;
  let s: string;
  if (typeof v === "number" && facet?.decimals != null) s = v.toFixed(facet.decimals);
  else s = String(v);
  return facet?.unit ? `${s} ${facet.unit}` : s;
};

interface Cell {
  uid: number;
  label: string;
}
interface RowCells {
  inputs: Cell[];
  outputs: Cell[];
  config: Cell[];
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

  // Split a component's user-facing props into Inputs/Outputs/Config, each
  // ordered by facet order then name. Hidden props excluded unless the toggle is
  // on. Each cell self-labels with the facet label (fallback raw name).
  const cellsFor = (c: Component): RowCells => {
    const facet = facets.get(c.uid);
    const buckets: Record<number, (Cell & { name: string; order: number })[]> = {
      [CATEGORY_INPUT]: [],
      [CATEGORY_OUTPUT]: [],
      [CATEGORY_CONFIG]: [],
    };
    for (const [name, p] of Object.entries(c.properties)) {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      const f = facet?.get(p.uid);
      if (!showHidden && f?.hidden) continue;
      const b = buckets[p.category];
      if (b) b.push({ uid: p.uid, label: f?.label || name, name, order: f?.order ?? 1e9 });
    }
    const sortB = (arr: (Cell & { name: string; order: number })[]) =>
      arr.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
    return {
      inputs: sortB(buckets[CATEGORY_INPUT]),
      outputs: sortB(buckets[CATEGORY_OUTPUT]),
      config: sortB(buckets[CATEGORY_CONFIG]),
    };
  };

  // Column counts = the widest row in each category (stable across the search so
  // columns don't jump as you filter).
  const { maxIn, maxOut, maxCfg } = useMemo(() => {
    let i = 0;
    let o = 0;
    let g = 0;
    for (const c of allRows) {
      const r = cellsFor(c);
      i = Math.max(i, r.inputs.length);
      o = Math.max(o, r.outputs.length);
      g = Math.max(g, r.config.length);
    }
    return { maxIn: i, maxOut: o, maxCfg: g };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [allRows, showHidden, facets]);

  const q = query.trim().toLowerCase();
  const matches = (c: Component) =>
    !q || (c.name || c.type).toLowerCase().includes(q) || c.type.toLowerCase().includes(q);

  const rows = useMemo(
    () =>
      allRows.filter(matches).sort((a, b) => (a.name || a.type).localeCompare(b.name || b.type) * dir),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allRows, q, dir],
  );

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
  const orphans = q ? allRows.filter((c) => sel.has(c.uid) && !matches(c)) : [];
  const totalCols = 1 + maxIn + maxOut + maxCfg;

  const valueCell = (cell: Cell | undefined, compUid: number, input: boolean, groupStart: boolean) => (
    <td
      key={`${compUid}:${cell?.uid ?? "_"}:${input}`}
      style={{
        padding: "4px 8px",
        whiteSpace: "nowrap",
        borderLeft: groupStart ? "1px solid #2c313c" : undefined,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      {cell && (
        <span style={{ display: "inline-flex", gap: 5, alignItems: "baseline" }}>
          <span style={{ color: "#5a6172" }}>{cell.label}</span>
          <span style={{ color: input ? "#cbd3e0" : "#e6e8eb" }}>
            {fmtCell(values[cell.uid], facets.get(compUid)?.get(cell.uid))}
          </span>
        </span>
      )}
    </td>
  );

  const renderRow = (c: Component) => {
    const r = cellsFor(c);
    const isFolder = (c.childrenCount ?? 0) > 0;
    return (
      <tr
        key={c.uid}
        data-uid={c.uid}
        onClick={(e) => onSelectRow(c.uid, e.shiftKey || e.metaKey || e.ctrlKey)}
        onDoubleClick={() => isFolder && onDrillIn(c.uid)}
        style={{
          cursor: "pointer",
          background: sel.has(c.uid) ? "#2c3a55" : "transparent",
          borderBottom: "1px solid #1f232b",
        }}
      >
        <td style={{ padding: "4px 10px", fontFamily: "ui-monospace, SFMono-Regular, monospace" }}>
          <span style={{ display: "flex", alignItems: "center", gap: 4, fontWeight: 600 }}>
            {isFolder && <Layers size={12} color="#9ecbff" />}
            <span style={{ color: "#e6e8eb" }}>{c.name || c.type}</span>
            {isFolder && <ChevronRight size={12} color="#5a6172" />}
          </span>
        </td>
        {Array.from({ length: maxIn }, (_, i) => valueCell(r.inputs[i], c.uid, true, i === 0))}
        {Array.from({ length: maxOut }, (_, i) => valueCell(r.outputs[i], c.uid, false, i === 0))}
        {Array.from({ length: maxCfg }, (_, i) => valueCell(r.config[i], c.uid, false, i === 0))}
      </tr>
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
          <table style={{ borderCollapse: "collapse", whiteSpace: "nowrap" }}>
            <thead>
              <tr style={{ position: "sticky", top: 0, background: "#1a1d24", zIndex: 1 }}>
                <GroupTh />
                {maxIn > 0 && <GroupTh span={maxIn}>Inputs</GroupTh>}
                {maxOut > 0 && <GroupTh span={maxOut}>Outputs</GroupTh>}
                {maxCfg > 0 && <GroupTh span={maxCfg}>Config</GroupTh>}
              </tr>
            </thead>
            <tbody>
              {rows.map(renderRow)}
              {orphans.length > 0 && (
                <>
                  <tr>
                    <td
                      colSpan={totalCols}
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
                    </td>
                  </tr>
                  {orphans.map(renderRow)}
                </>
              )}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function GroupTh({ children, span }: { children?: React.ReactNode; span?: number }) {
  return (
    <th
      colSpan={span}
      style={{
        textAlign: "left",
        padding: "4px 10px",
        borderBottom: "1px solid #2c313c",
        borderLeft: children ? "1px solid #2c313c" : undefined,
        color: "#8892a0",
        fontWeight: 600,
        fontSize: 10,
        textTransform: "uppercase",
        letterSpacing: 0.4,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      {children}
    </th>
  );
}
