import { useEffect, useMemo, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues, propertyDataType } from "../lib/store";
import { ROLE_NORMAL, ROLE_STATUS, CATEGORY_INPUT, CATEGORY_OUTPUT } from "../lib/engine-types";
import type { Component } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import { facetFor, rawFacet, aliasLabel, type PropFacet } from "../lib/facet";
import { ChevronRight, Layers } from "lucide-react";

// Tabular view of the CURRENT folder's components (step 1: read-only + live).
// Rows = direct children; columns = the union of their user-facing props
// (by name), with live values formatted via each row's facet. Selection is
// synced with the graph and a folder row drills in. Editing comes next.

const fmtCell = (v: DecodedValue | undefined, facet: PropFacet | undefined): string => {
  if (v === undefined || v === null) return "—";
  const al = aliasLabel(facet?.aliases, v);
  if (al) return al;
  let s: string;
  if (typeof v === "number" && facet?.decimals != null) s = v.toFixed(facet.decimals);
  else s = String(v);
  return facet?.unit ? `${s} ${facet.unit}` : s;
};

// Category sort weight for ordering columns: inputs, then outputs, then config.
const catRank = (c: number) => (c === CATEGORY_INPUT ? 0 : c === CATEGORY_OUTPUT ? 1 : 2);

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
  const components = useStructural((s) => s.components);
  const rows = useMemo(
    () =>
      Array.from(components.values())
        .filter((c) => c.parent === currentParentUid)
        .sort((a, b) => (a.name || a.type).localeCompare(b.name || b.type)),
    [components, currentParentUid],
  );

  // Per-component parsed facet (labels/units/aliases/hidden), cached by raw string.
  const facets = useMemo(
    () => new Map(rows.map((c) => [c.uid, facetFor(c.uid, rawFacet(c.properties))])),
    [rows],
  );

  // A prop is shown for a component when it's user-facing and (showHidden ||
  // not hidden in that component's facet).
  const visibleProps = (c: Component) =>
    Object.entries(c.properties).filter(([, p]) => {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) return false;
      if (showHidden) return true;
      return !facets.get(c.uid)?.get(p.uid)?.hidden;
    });

  // Union of prop NAMES across rows → columns, ordered input→output→config then
  // by name. Each column records a representative category for ordering.
  const columns = useMemo(() => {
    const byName = new Map<string, number>(); // name → category
    for (const c of rows) {
      for (const [name, p] of visibleProps(c)) {
        if (!byName.has(name)) byName.set(name, p.category);
      }
    }
    return [...byName.entries()]
      .map(([name, category]) => ({ name, category }))
      .sort((a, b) => catRank(a.category) - catRank(b.category) || a.name.localeCompare(b.name));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, showHidden, facets]);

  // Live values for every cell + the status prop, selected in one shallow read.
  const watchUids = useMemo(() => {
    const set = new Set<number>();
    for (const c of rows) {
      for (const p of Object.values(c.properties)) {
        if ((p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL || p.systemRole === ROLE_STATUS) {
          set.add(p.uid);
        }
      }
    }
    return [...set];
  }, [rows]);
  const values = useValues(
    useShallow((s) => {
      const out: Record<number, DecodedValue | undefined> = {};
      for (const uid of watchUids) out[uid] = s.values.get(uid);
      return out;
    }),
  );

  // Tell the editor which components the table needs subscribed (so off-screen
  // rows still stream live values, unioned with the graph's viewport subs).
  useEffect(() => {
    onRowsChange(rows.map((c) => c.uid));
    return () => onRowsChange([]);
  }, [rows, onRowsChange]);

  const sel = new Set(selectedUids);

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
        <span style={{ fontWeight: 600 }}>Components</span>
        <span style={{ color: "#5a6172", fontSize: 11 }}>{rows.length}</span>
        <label
          style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4, color: "#8892a0" }}
        >
          <input type="checkbox" checked={showHidden} onChange={(e) => setShowHidden(e.target.checked)} />
          show hidden
        </label>
      </div>

      <div style={{ overflow: "auto", flex: 1 }}>
        {rows.length === 0 ? (
          <div style={{ padding: 12, color: "#5a6172" }}>no components in this folder</div>
        ) : (
          <table style={{ borderCollapse: "collapse", width: "100%", whiteSpace: "nowrap" }}>
            <thead>
              <tr style={{ position: "sticky", top: 0, background: "#1a1d24", zIndex: 1 }}>
                <Th>name</Th>
                <Th>type</Th>
                {columns.map((col) => (
                  <Th key={col.name} numeric>
                    {col.name}
                  </Th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((c) => {
                const facet = facets.get(c.uid);
                const isFolder = (c.childrenCount ?? 0) > 0;
                return (
                  <tr
                    key={c.uid}
                    onClick={(e) => onSelectRow(c.uid, e.shiftKey || e.metaKey || e.ctrlKey)}
                    onDoubleClick={() => isFolder && onDrillIn(c.uid)}
                    style={{
                      cursor: "pointer",
                      background: sel.has(c.uid) ? "#2c3a55" : "transparent",
                      borderBottom: "1px solid #232733",
                    }}
                  >
                    <Td>
                      <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
                        {isFolder && (
                          <span
                            title="Double-click to enter"
                            style={{ display: "flex", color: "#9ecbff" }}
                          >
                            <Layers size={12} />
                          </span>
                        )}
                        <span style={{ color: "#e6e8eb" }}>{c.name || c.type}</span>
                        {isFolder && (
                          <ChevronRight size={12} color="#5a6172" style={{ marginLeft: 2 }} />
                        )}
                      </span>
                    </Td>
                    <Td muted>{c.type}</Td>
                    {columns.map((col) => {
                      const p = c.properties[col.name];
                      if (!p) return <Td key={col.name} numeric muted />;
                      const f = facet?.get(p.uid);
                      const isInput = p.category === CATEGORY_INPUT;
                      return (
                        <Td key={col.name} numeric input={isInput}>
                          {fmtCell(values[p.uid], f)}
                        </Td>
                      );
                    })}
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function Th({ children, numeric }: { children?: React.ReactNode; numeric?: boolean }) {
  return (
    <th
      style={{
        textAlign: numeric ? "right" : "left",
        padding: "5px 10px",
        borderBottom: "1px solid #2c313c",
        color: "#8892a0",
        fontWeight: 600,
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      {children}
    </th>
  );
}

function Td({
  children,
  numeric,
  muted,
  input,
}: {
  children?: React.ReactNode;
  numeric?: boolean;
  muted?: boolean;
  input?: boolean;
}) {
  return (
    <td
      style={{
        textAlign: numeric ? "right" : "left",
        padding: "4px 10px",
        color: muted ? "#5a6172" : input ? "#cbd3e0" : "#e6e8eb",
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      {children}
    </td>
  );
}
