import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStructural, useValues, useStatusFlags, propertyDataType } from "../lib/store";
import {
  ROLE_NORMAL,
  ROLE_STATUS,
  CATEGORY_INPUT,
  CATEGORY_OUTPUT,
  CATEGORY_CONFIG,
  STATUS_OVERRIDDEN,
  DATATYPE_BOOL,
  DATATYPE_NUMBER,
} from "../lib/engine-types";
import type { Component, FlexValue } from "../lib/engine-types";
import type { DecodedValue } from "../lib/wire";
import { facetFor, rawFacet, aliasLabel, type PropFacet, type ComponentFacet } from "../lib/facet";
import { Layers, ArrowUp, ArrowDown, FolderUp } from "lucide-react";

// Table view of the CURRENT folder's components. Each component is a row; its
// props align into per-slot columns under Inputs / Outputs / Config category
// headers (no per-prop headers — each cell self-labels with the facet label).
// Input cells are editable (commit sets an override); outputs are read-only.
// Double-click a row centers the wiresheet on that component.

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
  name: string;
  label: string;
  category: number;
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
  onCenter,
  onRowsChange,
  onSetOverride,
  onClearOverride,
  canGoUp,
  onUp,
}: {
  currentParentUid: number;
  selectedUids: number[];
  onSelectRow: (uid: number, additive: boolean) => void;
  onDrillIn: (uid: number) => void;
  onCenter: (uid: number) => void;
  onRowsChange: (uids: number[]) => void;
  onSetOverride: (componentUid: number, property: string, value: FlexValue) => void;
  onClearOverride: (componentUid: number, property: string) => void;
  canGoUp: boolean;
  onUp: () => void;
}) {
  const [showHidden, setShowHidden] = useState(false);
  const [query, setQuery] = useState("");
  const [dir, setDir] = useState<1 | -1>(1);
  const [editing, setEditing] = useState<{ comp: number; uid: number } | null>(null);
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

  const cellsFor = (c: Component): RowCells => {
    const facet = facets.get(c.uid);
    const buckets: Record<number, (Cell & { order: number })[]> = {
      [CATEGORY_INPUT]: [],
      [CATEGORY_OUTPUT]: [],
      [CATEGORY_CONFIG]: [],
    };
    for (const [name, p] of Object.entries(c.properties)) {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      const f = facet?.get(p.uid);
      if (!showHidden && f?.hidden) continue;
      const b = buckets[p.category];
      if (b) b.push({ uid: p.uid, name, label: f?.label || name, category: p.category, order: f?.order ?? 1e9 });
    }
    const sortB = (arr: (Cell & { order: number })[]) =>
      arr.sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
    return {
      inputs: sortB(buckets[CATEGORY_INPUT]),
      outputs: sortB(buckets[CATEGORY_OUTPUT]),
      config: sortB(buckets[CATEGORY_CONFIG]),
    };
  };

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
  const flags = useStatusFlags(
    useShallow((s) => {
      const out: Record<number, number> = {};
      for (const uid of watchUids) out[uid] = s.flags.get(uid) ?? 0;
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

  const valueCell = (cell: Cell | undefined, c: Component, groupStart: boolean) => {
    const editable = cell != null && cell.category === CATEGORY_INPUT;
    const overridden = cell != null && (flags[cell.uid] & STATUS_OVERRIDDEN) !== 0;
    const isEditing = cell != null && editing?.comp === c.uid && editing.uid === cell.uid;
    return (
      <td
        key={`${c.uid}:${cell?.uid ?? "_"}:${cell?.category ?? 0}`}
        onContextMenu={(e) => {
          if (overridden && cell) {
            e.preventDefault();
            e.stopPropagation();
            onClearOverride(c.uid, cell.name);
          }
        }}
        style={{
          padding: "4px 8px",
          whiteSpace: "nowrap",
          borderLeft: groupStart ? "1px solid #2c313c" : undefined,
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
        }}
      >
        {cell &&
          (isEditing ? (
            <Editor
              initial={values[cell.uid]}
              dataType={propertyDataType.get(cell.uid) ?? DATATYPE_NUMBER}
              facet={facets.get(c.uid)?.get(cell.uid)}
              onCommit={(v) => {
                onSetOverride(c.uid, cell.name, v);
                setEditing(null);
              }}
              onCancel={() => setEditing(null)}
            />
          ) : (
            <span style={{ display: "inline-flex", gap: 5, alignItems: "baseline" }}>
              <span style={{ color: "#5a6172" }}>{cell.label}</span>
              <span
                onClick={
                  editable
                    ? (e) => {
                        e.stopPropagation();
                        setEditing({ comp: c.uid, uid: cell.uid });
                      }
                    : undefined
                }
                title={overridden ? "overridden — right-click to clear" : editable ? "click to set" : undefined}
                style={{
                  color: overridden ? "#ffd166" : cell.category === CATEGORY_INPUT ? "#cbd3e0" : "#e6e8eb",
                  cursor: editable ? "text" : "default",
                  borderBottom: editable ? "1px dotted #3b4350" : undefined,
                }}
              >
                {fmtCell(values[cell.uid], facets.get(c.uid)?.get(cell.uid))}
              </span>
            </span>
          ))}
      </td>
    );
  };

  const renderRow = (c: Component) => {
    const r = cellsFor(c);
    const isFolder = (c.childrenCount ?? 0) > 0;
    return (
      <tr
        key={c.uid}
        data-uid={c.uid}
        // Plain click = select; Ctrl/Cmd-click = focus the graph on it;
        // Shift-click = add to selection; double-click = go inside (works even
        // for childless components — drills to an empty level).
        onClick={(e) => {
          if (e.metaKey || e.ctrlKey) onCenter(c.uid);
          else onSelectRow(c.uid, e.shiftKey);
        }}
        onDoubleClick={() => onDrillIn(c.uid)}
        title="double-click to go inside · ctrl-click to focus on canvas"
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
          </span>
        </td>
        {Array.from({ length: maxIn }, (_, i) => valueCell(r.inputs[i], c, i === 0))}
        {Array.from({ length: maxOut }, (_, i) => valueCell(r.outputs[i], c, i === 0))}
        {Array.from({ length: maxCfg }, (_, i) => valueCell(r.config[i], c, i === 0))}
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
        <button
          onClick={onUp}
          disabled={!canGoUp}
          title="Up to parent folder"
          style={{
            display: "flex",
            alignItems: "center",
            background: "transparent",
            border: "1px solid #2c313c",
            borderRadius: 3,
            color: canGoUp ? "#cbd3e0" : "#3b4350",
            cursor: canGoUp ? "pointer" : "default",
            padding: "3px 6px",
            flexShrink: 0,
          }}
        >
          <FolderUp size={14} />
        </button>
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

// Inline value editor — a dropdown for aliased / boolean props, else a text
// input. Enter or blur commits, Esc cancels.
function Editor({
  initial,
  dataType,
  facet,
  onCommit,
  onCancel,
}: {
  initial: DecodedValue | undefined;
  dataType: number;
  facet: PropFacet | undefined;
  onCommit: (v: FlexValue) => void;
  onCancel: () => void;
}) {
  const aliases = facet?.aliases;
  const codeOf = (v: DecodedValue | undefined) =>
    v === true ? 1 : v === false ? 0 : typeof v === "number" ? v : Number(v);
  const initStr =
    aliases?.length || dataType === DATATYPE_BOOL
      ? String(codeOf(initial))
      : initial == null
        ? ""
        : String(initial);
  const [text, setText] = useState(initStr);
  const ref = useRef<HTMLInputElement | HTMLSelectElement>(null);
  useEffect(() => {
    ref.current?.focus();
    if (ref.current instanceof HTMLInputElement) ref.current.select();
  }, []);

  const coerce = (raw: string): FlexValue => {
    if (aliases?.length) {
      const code = Number(raw);
      return dataType === DATATYPE_BOOL ? code === 1 : code;
    }
    if (dataType === DATATYPE_BOOL) return raw === "1" || raw === "true";
    if (dataType === DATATYPE_NUMBER) return Number(raw);
    return raw;
  };

  const fieldStyle = {
    background: "#0f1115",
    color: "#e6e8eb",
    border: "1px solid #3b5388",
    borderRadius: 2,
    padding: "1px 4px",
    fontSize: 12,
    fontFamily: "ui-monospace, SFMono-Regular, monospace",
    outline: "none",
    width: 70,
  } as const;

  const onKey = (e: React.KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === "Enter") onCommit(coerce(text));
    else if (e.key === "Escape") onCancel();
  };

  if (aliases?.length || dataType === DATATYPE_BOOL) {
    const opts = aliases?.length
      ? aliases.map((a) => ({ v: String(a.code), label: a.label }))
      : [
          { v: "0", label: "false" },
          { v: "1", label: "true" },
        ];
    return (
      <select
        ref={ref as React.RefObject<HTMLSelectElement>}
        value={text}
        onChange={(e) => onCommit(coerce(e.target.value))}
        onKeyDown={onKey}
        onBlur={onCancel}
        style={{ ...fieldStyle, width: "auto" }}
      >
        {opts.map((o) => (
          <option key={o.v} value={o.v}>
            {o.label}
          </option>
        ))}
      </select>
    );
  }
  return (
    <input
      ref={ref as React.RefObject<HTMLInputElement>}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onKeyDown={onKey}
      onBlur={() => onCommit(coerce(text))}
      inputMode={dataType === DATATYPE_NUMBER ? "decimal" : "text"}
      style={fieldStyle}
    />
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
