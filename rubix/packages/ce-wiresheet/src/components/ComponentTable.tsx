import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues } from "../lib/store";
import { ROLE_NORMAL, ROLE_STATUS, CATEGORY_INPUT, CATEGORY_OUTPUT } from "../lib/engine-types";
import type { Component } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import {
  facetFor,
  rawFacet,
  aliasLabel,
  exposedPorts,
  type PropFacet,
  type ComponentFacet,
} from "../lib/facet";
import { Layers, ChevronRight, CornerDownRight, ArrowUp, ArrowDown } from "lucide-react";

// Flat list of the CURRENT folder's components in an ALIGNED grid: columns are
// the union of their user-facing props AND exposed ports (so values line up for
// scanning), grouped under "Inputs / Outputs / Config" header bands. Exposed
// ports carry a ↪ marker. Column headers use the facet label when every
// component agrees, else the raw prop name. Search covers names, types, prop
// names/labels and value aliases. A selected component the search filters out is
// pinned at the bottom.

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
const catLabel = (c: number) => (c === CATEGORY_INPUT ? "Inputs" : c === CATEGORY_OUTPUT ? "Outputs" : "Config");

// One displayable field of a component: an own prop, or an exposed child-prop port.
interface Field {
  key: string; // column id (prop name, or "↪label" for a port)
  uid: number; // value-stream uid (own prop uid, or the child prop uid)
  label: string;
  category: number;
  exposed: boolean;
  facet?: PropFacet;
}
interface Column {
  key: string;
  category: number;
  header: string;
  exposed: boolean;
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

  // Own visible props + exposed ports, as a unified field list.
  const fieldsFor = (c: Component): Field[] => {
    const facet = facets.get(c.uid);
    const out: Field[] = [];
    for (const [name, p] of Object.entries(c.properties)) {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      const f = facet?.get(p.uid);
      if (!showHidden && f?.hidden) continue;
      out.push({ key: name, uid: p.uid, label: f?.label || name, category: p.category, exposed: false, facet: f });
    }
    if (facet) {
      for (const ep of exposedPorts(facet)) {
        const label = ep.facet.label || `port ${ep.childUid}`;
        out.push({
          key: `↪${label}`,
          uid: ep.childUid,
          label,
          category: ep.side === "input" ? CATEGORY_INPUT : CATEGORY_OUTPUT,
          exposed: true,
          facet: ep.facet,
        });
      }
    }
    return out;
  };

  const q = query.trim().toLowerCase();
  const matches = (c: Component) => {
    if (!q) return true;
    if ((c.name || c.type).toLowerCase().includes(q) || c.type.toLowerCase().includes(q)) return true;
    for (const f of fieldsFor(c)) {
      if (f.label.toLowerCase().includes(q) || f.key.toLowerCase().includes(q)) return true;
      if (f.facet?.aliases?.some((a) => a.label.toLowerCase().includes(q))) return true;
    }
    return false;
  };

  const rows = useMemo(
    () =>
      allRows
        .filter(matches)
        .sort((a, b) => (a.name || a.type).localeCompare(b.name || b.type) * dir),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allRows, q, dir, showHidden, facets],
  );

  // Union columns across the visible rows, ordered input→output→config then name.
  const columns = useMemo<Column[]>(() => {
    const map = new Map<string, { category: number; labels: Set<string>; exposed: boolean }>();
    for (const c of rows) {
      for (const f of fieldsFor(c)) {
        let e = map.get(f.key);
        if (!e) {
          e = { category: f.category, labels: new Set(), exposed: f.exposed };
          map.set(f.key, e);
        }
        e.labels.add(f.label);
      }
    }
    return [...map.entries()]
      .map(([key, e]) => ({
        key,
        category: e.category,
        exposed: e.exposed,
        header: e.labels.size === 1 ? [...e.labels][0] : key.replace(/^↪/, ""),
      }))
      .sort((a, b) => catRank(a.category) - catRank(b.category) || a.header.localeCompare(b.header));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, showHidden, facets]);

  const bands = useMemo(() => {
    const out: { category: number; count: number }[] = [];
    for (const col of columns) {
      const last = out[out.length - 1];
      if (last && last.category === col.category) last.count++;
      else out.push({ category: col.category, count: 1 });
    }
    return out;
  }, [columns]);

  // Live values for own props + status + exposed child props (the latter are
  // already property-subscribed by the graph for the current folder's ports).
  const watchUids = useMemo(() => {
    const set = new Set<number>();
    for (const c of allRows) {
      for (const p of Object.values(c.properties))
        if ((p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL || p.systemRole === ROLE_STATUS) set.add(p.uid);
      for (const ep of exposedPorts(facets.get(c.uid) ?? new Map())) set.add(ep.childUid);
    }
    return [...set];
  }, [allRows, facets]);
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

  const renderRow = (c: Component) => {
    const isFolder = (c.childrenCount ?? 0) > 0;
    const byKey = new Map(fieldsFor(c).map((f) => [f.key, f]));
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
        <Td sticky>
          <span style={{ display: "flex", alignItems: "center", gap: 4, fontWeight: 600 }}>
            {isFolder && <Layers size={12} color="#9ecbff" />}
            <span style={{ color: "#e6e8eb" }}>{c.name || c.type}</span>
            {isFolder && <ChevronRight size={12} color="#5a6172" />}
          </span>
        </Td>
        {columns.map((col) => {
          const f = byKey.get(col.key);
          if (!f) return <Td key={col.key} numeric muted />;
          return (
            <Td key={col.key} numeric input={f.category === CATEGORY_INPUT}>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 3, justifyContent: "flex-end" }}>
                {f.exposed && <CornerDownRight size={10} color="#7a8a9f" />}
                {fmtCell(values[f.uid], f.facet)}
              </span>
            </Td>
          );
        })}
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
          placeholder="search name, prop, alias…"
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
          <table style={{ borderCollapse: "collapse", width: "100%", whiteSpace: "nowrap" }}>
            <thead>
              <tr style={{ position: "sticky", top: 0, zIndex: 2 }}>
                <BandTh sticky />
                {bands.map((b, i) => (
                  <BandTh key={i} span={b.count}>
                    {catLabel(b.category)}
                  </BandTh>
                ))}
              </tr>
              <tr style={{ position: "sticky", top: 22, zIndex: 2 }}>
                <ColTh sticky>name</ColTh>
                {columns.map((col) => (
                  <ColTh key={col.key} title={col.key.replace(/^↪/, "")} exposed={col.exposed}>
                    {col.header}
                  </ColTh>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map(renderRow)}
              {orphans.length > 0 && (
                <>
                  <tr>
                    <td
                      colSpan={columns.length + 1}
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

function BandTh({ children, span, sticky }: { children?: React.ReactNode; span?: number; sticky?: boolean }) {
  return (
    <th
      colSpan={span}
      style={{
        textAlign: "center",
        padding: "3px 10px",
        background: "#181c23",
        borderBottom: "1px solid #2c313c",
        borderLeft: span ? "1px solid #2c313c" : undefined,
        color: "#7a8aa0",
        fontWeight: 600,
        fontSize: 9,
        textTransform: "uppercase",
        letterSpacing: 0.5,
        ...(sticky ? { position: "sticky", left: 0, zIndex: 1, textAlign: "left" } : {}),
      }}
    >
      {children}
    </th>
  );
}

function ColTh({
  children,
  sticky,
  title,
  exposed,
}: {
  children?: React.ReactNode;
  sticky?: boolean;
  title?: string;
  exposed?: boolean;
}) {
  return (
    <th
      title={title}
      style={{
        textAlign: sticky ? "left" : "right",
        padding: "4px 10px",
        background: "#1a1d24",
        borderBottom: "1px solid #2c313c",
        color: "#8892a0",
        fontWeight: 600,
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        ...(sticky ? { position: "sticky", left: 0, zIndex: 1 } : {}),
      }}
    >
      <span style={{ display: "inline-flex", alignItems: "center", gap: 3, justifyContent: "flex-end" }}>
        {exposed && <CornerDownRight size={10} color="#7a8a9f" />}
        {children}
      </span>
    </th>
  );
}

function Td({
  children,
  numeric,
  muted,
  input,
  sticky,
}: {
  children?: React.ReactNode;
  numeric?: boolean;
  muted?: boolean;
  input?: boolean;
  sticky?: boolean;
}) {
  return (
    <td
      style={{
        textAlign: numeric ? "right" : "left",
        padding: "4px 10px",
        color: muted ? "#5a6172" : input ? "#cbd3e0" : "#e6e8eb",
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        ...(sticky ? { position: "sticky", left: 0, background: "inherit", zIndex: 1 } : {}),
      }}
    >
      {children}
    </td>
  );
}
