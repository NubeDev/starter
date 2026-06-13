import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues } from "../lib/store";
import { ROLE_NORMAL, ROLE_STATUS, CATEGORY_INPUT, CATEGORY_OUTPUT } from "../lib/engine-types";
import type { Component } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import { facetFor, rawFacet, aliasLabel, type PropFacet, type ComponentFacet } from "../lib/facet";
import { ChevronRight, ChevronDown, Layers, ArrowUp, ArrowDown } from "lucide-react";

// Tabular view of the CURRENT folder's components, GROUPED BY TYPE. Each type is
// its own section + mini-table with only that type's columns — so a folder of
// 100 mixed components reads as tidy per-type tables instead of one giant sparse
// grid. Live values, search, per-column sort; a selected component that the
// search filters out still appears in a pinned section at the bottom.

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
const shortType = (t: string) => t.split("::").pop() || t;

interface Sort {
  col: string; // "name" or a prop name
  dir: 1 | -1;
}
interface Group {
  type: string;
  members: Component[];
  columns: { name: string; category: number }[];
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
  const [sort, setSort] = useState<Sort>({ col: "name", dir: 1 });
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const scrollRef = useRef<HTMLDivElement>(null);

  const components = useStructural((s) => s.components);
  const rows = useMemo(
    () => Array.from(components.values()).filter((c) => c.parent === currentParentUid),
    [components, currentParentUid],
  );

  const facets = useMemo(
    () => new Map<number, ComponentFacet>(rows.map((c) => [c.uid, facetFor(c.uid, rawFacet(c.properties))])),
    [rows],
  );

  const visibleProps = (c: Component) =>
    Object.entries(c.properties).filter(([, p]) => {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) return false;
      if (showHidden) return true;
      return !facets.get(c.uid)?.get(p.uid)?.hidden;
    });

  const q = query.trim().toLowerCase();
  const matches = (c: Component) =>
    !q || (c.name || c.type).toLowerCase().includes(q) || c.type.toLowerCase().includes(q);

  // Group matching rows by type; build each group's columns (homogeneous, so they
  // align). Structural only (no live values) → stable, recomputed on edits.
  const groups = useMemo(() => {
    const byType = new Map<string, Component[]>();
    for (const c of rows) {
      if (!matches(c)) continue;
      const arr = byType.get(c.type);
      if (arr) arr.push(c);
      else byType.set(c.type, [c]);
    }
    const out: Group[] = [];
    for (const [type, members] of byType) {
      const colMap = new Map<string, number>();
      for (const c of members) for (const [name, p] of visibleProps(c)) if (!colMap.has(name)) colMap.set(name, p.category);
      const columns = [...colMap.entries()]
        .map(([name, category]) => ({ name, category }))
        .sort((a, b) => catRank(a.category) - catRank(b.category) || a.name.localeCompare(b.name));
      out.push({ type, members, columns });
    }
    return out.sort((a, b) => shortType(a.type).localeCompare(shortType(b.type)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, q, showHidden, facets]);

  // Live values for every cell + status, in one shallow read.
  const watchUids = useMemo(() => {
    const set = new Set<number>();
    for (const c of rows)
      for (const p of Object.values(c.properties))
        if ((p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL || p.systemRole === ROLE_STATUS) set.add(p.uid);
    return [...set];
  }, [rows]);
  const values = useValues(
    useShallow((s) => {
      const out: Record<number, DecodedValue | undefined> = {};
      for (const uid of watchUids) out[uid] = s.values.get(uid);
      return out;
    }),
  );

  useEffect(() => {
    onRowsChange(rows.map((c) => c.uid));
    return () => onRowsChange([]);
  }, [rows, onRowsChange]);

  // Scroll the (first) selected row into view when the selection changes.
  const firstSel = selectedUids[0];
  useEffect(() => {
    if (firstSel == null) return;
    scrollRef.current
      ?.querySelector<HTMLElement>(`[data-uid="${firstSel}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [firstSel, groups]);

  const sel = new Set(selectedUids);

  // Sort a group's members by the active column (live value for prop columns).
  const sortKey = (c: Component): string | number | undefined => {
    if (sort.col === "name") return (c.name || c.type).toLowerCase();
    const p = c.properties[sort.col];
    return p ? (values[p.uid] as string | number | undefined) : undefined;
  };
  const sortMembers = (members: Component[]) => {
    const arr = [...members];
    arr.sort((a, b) => {
      const ka = sortKey(a);
      const kb = sortKey(b);
      if (ka === undefined && kb === undefined) return 0;
      if (ka === undefined) return 1; // undefined always last
      if (kb === undefined) return -1;
      const c =
        typeof ka === "number" && typeof kb === "number"
          ? ka - kb
          : String(ka).localeCompare(String(kb));
      return c * sort.dir;
    });
    return arr;
  };
  const onSort = (col: string) =>
    setSort((s) => (s.col === col ? { col, dir: s.dir === 1 ? -1 : 1 } : { col, dir: 1 }));

  // Selected components the search filtered out — pinned at the bottom so the
  // selection is never lost behind a filter.
  const orphans = q ? rows.filter((c) => sel.has(c.uid) && !matches(c)) : [];

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
        <label style={{ display: "flex", alignItems: "center", gap: 4, color: "#8892a0", flexShrink: 0 }}>
          <input type="checkbox" checked={showHidden} onChange={(e) => setShowHidden(e.target.checked)} />
          hidden
        </label>
      </div>

      <div ref={scrollRef} style={{ overflow: "auto", flex: 1 }}>
        {groups.length === 0 && orphans.length === 0 ? (
          <div style={{ padding: 12, color: "#5a6172" }}>
            {rows.length === 0 ? "no components in this folder" : "no matches"}
          </div>
        ) : (
          groups.map((g) => {
            const isCollapsed = collapsed.has(g.type);
            return (
              <div key={g.type}>
                <button
                  onClick={() =>
                    setCollapsed((cur) => {
                      const next = new Set(cur);
                      if (next.has(g.type)) next.delete(g.type);
                      else next.add(g.type);
                      return next;
                    })
                  }
                  title={g.type}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 5,
                    width: "100%",
                    textAlign: "left",
                    background: "#1a1d24",
                    border: "none",
                    borderBottom: "1px solid #2c313c",
                    color: "#cbd3e0",
                    cursor: "pointer",
                    padding: "5px 10px",
                    position: "sticky",
                    top: 0,
                    zIndex: 1,
                    fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    fontSize: 11,
                  }}
                >
                  {isCollapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
                  <span style={{ fontWeight: 600 }}>{shortType(g.type)}</span>
                  <span style={{ color: "#5a6172" }}>{g.members.length}</span>
                </button>
                {!isCollapsed && (
                  <table style={{ borderCollapse: "collapse", width: "100%", whiteSpace: "nowrap" }}>
                    <thead>
                      <tr>
                        <Th sortable active={sort.col === "name"} dir={sort.dir} onClick={() => onSort("name")}>
                          name
                        </Th>
                        {g.columns.map((col) => (
                          <Th
                            key={col.name}
                            numeric
                            sortable
                            active={sort.col === col.name}
                            dir={sort.dir}
                            onClick={() => onSort(col.name)}
                          >
                            {col.name}
                          </Th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {sortMembers(g.members).map((c) => {
                        const facet = facets.get(c.uid);
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
                              borderBottom: "1px solid #232733",
                            }}
                          >
                            <Td>
                              <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
                                {isFolder && <Layers size={12} color="#9ecbff" />}
                                <span style={{ color: "#e6e8eb" }}>{c.name || c.type}</span>
                                {isFolder && <ChevronRight size={12} color="#5a6172" />}
                              </span>
                            </Td>
                            {g.columns.map((col) => {
                              const p = c.properties[col.name];
                              if (!p) return <Td key={col.name} numeric muted />;
                              return (
                                <Td key={col.name} numeric input={p.category === CATEGORY_INPUT}>
                                  {fmtCell(values[p.uid], facet?.get(p.uid))}
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
            );
          })
        )}

        {orphans.length > 0 && (
          <div>
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
            <table style={{ borderCollapse: "collapse", width: "100%", whiteSpace: "nowrap" }}>
              <tbody>
                {orphans.map((c) => (
                  <tr
                    key={c.uid}
                    data-uid={c.uid}
                    onClick={(e) => onSelectRow(c.uid, e.shiftKey || e.metaKey || e.ctrlKey)}
                    style={{
                      cursor: "pointer",
                      background: "#2c3a55",
                      borderBottom: "1px solid #232733",
                    }}
                  >
                    <Td>{c.name || c.type}</Td>
                    <Td muted>{shortType(c.type)}</Td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

function Th({
  children,
  numeric,
  sortable,
  active,
  dir,
  onClick,
}: {
  children?: React.ReactNode;
  numeric?: boolean;
  sortable?: boolean;
  active?: boolean;
  dir?: 1 | -1;
  onClick?: () => void;
}) {
  return (
    <th
      onClick={onClick}
      style={{
        textAlign: numeric ? "right" : "left",
        padding: "4px 10px",
        borderBottom: "1px solid #2c313c",
        color: active ? "#cbd3e0" : "#8892a0",
        fontWeight: 600,
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        cursor: sortable ? "pointer" : "default",
        userSelect: "none",
        whiteSpace: "nowrap",
      }}
    >
      <span
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 2,
          flexDirection: numeric ? "row-reverse" : "row",
        }}
      >
        {children}
        {active && (dir === 1 ? <ArrowUp size={11} /> : <ArrowDown size={11} />)}
      </span>
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
