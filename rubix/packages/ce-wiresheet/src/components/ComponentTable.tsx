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
  onSelect,
  onDrillIn,
  onRowsChange,
  onSetOverride,
  onSetDefault,
  onClearOverride,
  canGoUp,
  onUp,
}: {
  currentParentUid: number;
  selectedUids: number[];
  onSelect: (uids: number[]) => void; // replaces the selection with these uids
  onDrillIn: (uid: number) => void;
  onRowsChange: (uids: number[]) => void;
  onSetOverride: (componentUid: number, property: string, value: FlexValue, duration: number) => void;
  onSetDefault: (componentUid: number, property: string, value: FlexValue) => void;
  onClearOverride: (componentUid: number, property: string) => void;
  canGoUp: boolean;
  onUp: () => void;
}) {
  const [showHidden, setShowHidden] = useState(false);
  const [query, setQuery] = useState("");
  const [dir, setDir] = useState<1 | -1>(1);
  const [editing, setEditing] = useState<{ comp: number; cell: Cell; rect: DOMRect } | null>(null);
  const [anchor, setAnchor] = useState<number | null>(null);
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

  // Displayed row order (incl. pinned orphans) — used for shift-range selection.
  const orderedUids = useMemo(() => [...rows, ...orphans].map((c) => c.uid), [rows, orphans]);

  // Selection gestures: plain = single; ctrl/cmd = toggle; shift = range from the
  // anchor (last plain/toggle click) to the clicked row, inclusive.
  const handleRowClick = (uid: number, e: React.MouseEvent) => {
    if (e.shiftKey && anchor != null) {
      const a = orderedUids.indexOf(anchor);
      const b = orderedUids.indexOf(uid);
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a <= b ? [a, b] : [b, a];
        onSelect(orderedUids.slice(lo, hi + 1));
        return;
      }
    }
    if (e.metaKey || e.ctrlKey) {
      const next = new Set(selectedUids);
      if (next.has(uid)) next.delete(uid);
      else next.add(uid);
      onSelect([...next]);
      setAnchor(uid);
      return;
    }
    onSelect([uid]);
    setAnchor(uid);
  };

  const valueCell = (cell: Cell | undefined, c: Component, groupStart: boolean, slot: string) => {
    const overridden = cell != null && (flags[cell.uid] & STATUS_OVERRIDDEN) !== 0;
    return (
      <td
        key={slot}
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
        {cell && (
          <span style={{ display: "inline-flex", gap: 5, alignItems: "baseline" }}>
            <span style={{ color: "#5a6172" }}>{cell.label}</span>
            <span
              onClick={(e) => {
                e.stopPropagation();
                setEditing({ comp: c.uid, cell, rect: e.currentTarget.getBoundingClientRect() });
              }}
              title={overridden ? "overridden — right-click to clear · click to edit" : "click to edit"}
              style={{
                color: cell.category === CATEGORY_INPUT ? "#cbd3e0" : "#e6e8eb",
                cursor: "pointer",
                borderBottom: "1px dotted #3b4350",
              }}
            >
              {fmtCell(values[cell.uid], facets.get(c.uid)?.get(cell.uid))}
            </span>
            {overridden && <OvrBadge />}
          </span>
        )}
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
        // Plain = select; Ctrl/Cmd = toggle; Shift = range from the anchor;
        // double-click = go inside; Space (editor level) focuses the canvas.
        onClick={(e) => handleRowClick(c.uid, e)}
        onDoubleClick={() => onDrillIn(c.uid)}
        title="double-click to go inside · ctrl-click to toggle · shift-click to range-select · space to focus on canvas"
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
        {Array.from({ length: maxIn }, (_, i) => valueCell(r.inputs[i], c, i === 0, `i${i}`))}
        {Array.from({ length: maxOut }, (_, i) => valueCell(r.outputs[i], c, i === 0, `o${i}`))}
        {Array.from({ length: maxCfg }, (_, i) => valueCell(r.config[i], c, i === 0, `g${i}`))}
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

      {editing && (
        <ValueEditor
          rect={editing.rect}
          cell={editing.cell}
          initial={values[editing.cell.uid]}
          dataType={propertyDataType.get(editing.cell.uid) ?? DATATYPE_NUMBER}
          facet={facets.get(editing.comp)?.get(editing.cell.uid)}
          overridden={(flags[editing.cell.uid] & STATUS_OVERRIDDEN) !== 0}
          onOverride={(v, duration) => {
            onSetOverride(editing.comp, editing.cell.name, v, duration);
            setEditing(null);
          }}
          onDefault={(v) => {
            onSetDefault(editing.comp, editing.cell.name, v);
            setEditing(null);
          }}
          onClear={() => {
            onClearOverride(editing.comp, editing.cell.name);
            setEditing(null);
          }}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  );
}

// Popover value editor. Inputs/config can be applied as an OVERRIDE (with a
// duration) or written as the stored DEFAULT; outputs are override-only. Aliased
// / boolean props edit as a dropdown, else a text/number field.
function ValueEditor({
  rect,
  cell,
  initial,
  dataType,
  facet,
  overridden,
  onOverride,
  onDefault,
  onClear,
  onClose,
}: {
  rect: DOMRect;
  cell: Cell;
  initial: DecodedValue | undefined;
  dataType: number;
  facet: PropFacet | undefined;
  overridden: boolean;
  onOverride: (v: FlexValue, duration: number) => void;
  onDefault: (v: FlexValue) => void;
  onClear: () => void;
  onClose: () => void;
}) {
  const aliases = facet?.aliases;
  const isOutput = cell.category === CATEGORY_OUTPUT;
  const codeOf = (v: DecodedValue | undefined) =>
    v === true ? 1 : v === false ? 0 : typeof v === "number" ? v : Number(v);
  const initStr =
    aliases?.length || dataType === DATATYPE_BOOL
      ? String(codeOf(initial))
      : initial == null
        ? ""
        : String(initial);
  const [text, setText] = useState(initStr);
  const [mode, setMode] = useState<"override" | "default">("override");
  // Default 60s — matching the standard override menu. (duration 0 / "permanent"
  // does not actually hold on this engine, so it can't be the default.)
  const [duration, setDuration] = useState("60");
  const ref = useRef<HTMLInputElement | HTMLSelectElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ref.current?.focus();
    if (ref.current instanceof HTMLInputElement) ref.current.select();
  }, []);
  useEffect(() => {
    const dismiss = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    return () => document.removeEventListener("pointerdown", dismiss, true);
  }, [onClose]);

  const coerce = (raw: string): FlexValue => {
    if (aliases?.length) {
      const code = Number(raw);
      return dataType === DATATYPE_BOOL ? code === 1 : code;
    }
    if (dataType === DATATYPE_BOOL) return raw === "1" || raw === "true";
    if (dataType === DATATYPE_NUMBER) return Number(raw);
    return raw;
  };
  const apply = () => {
    const v = coerce(text);
    if (mode === "default" && !isOutput) onDefault(v);
    else onOverride(v, Number(duration) || 0);
  };

  const field = {
    background: "#0f1115",
    color: "#e6e8eb",
    border: "1px solid #2c313c",
    borderRadius: 3,
    padding: "3px 6px",
    fontSize: 12,
    fontFamily: "ui-monospace, SFMono-Regular, monospace",
    outline: "none",
  } as const;
  const segBtn = (active: boolean) =>
    ({
      flex: 1,
      background: active ? "#2c3a55" : "transparent",
      color: active ? "#cfe0ff" : "#8892a0",
      border: "1px solid #2c313c",
      borderRadius: 3,
      padding: "3px 6px",
      cursor: "pointer",
      fontSize: 11,
    }) as const;

  const left = Math.min(rect.left, window.innerWidth - 220);
  const top = Math.min(rect.bottom + 4, window.innerHeight - 160);

  return (
    <div
      ref={rootRef}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Escape") onClose();
        else if (e.key === "Enter" && !(e.target instanceof HTMLSelectElement)) apply();
      }}
      style={{
        position: "fixed",
        left,
        top,
        zIndex: 200,
        width: 200,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 6,
        boxShadow: "0 8px 24px rgba(0,0,0,0.6)",
        padding: 8,
        display: "flex",
        flexDirection: "column",
        gap: 6,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        fontSize: 12,
      }}
    >
      <div style={{ color: "#9ecbff", fontFamily: "ui-monospace, monospace", fontSize: 11 }}>
        {cell.label}
      </div>

      {aliases?.length || dataType === DATATYPE_BOOL ? (
        <select
          ref={ref as React.RefObject<HTMLSelectElement>}
          value={text}
          onChange={(e) => setText(e.target.value)}
          style={field}
        >
          {(aliases?.length
            ? aliases.map((a) => ({ v: String(a.code), label: a.label }))
            : [
                { v: "0", label: "false" },
                { v: "1", label: "true" },
              ]
          ).map((o) => (
            <option key={o.v} value={o.v}>
              {o.label}
            </option>
          ))}
        </select>
      ) : (
        <input
          ref={ref as React.RefObject<HTMLInputElement>}
          value={text}
          onChange={(e) => setText(e.target.value)}
          inputMode={dataType === DATATYPE_NUMBER ? "decimal" : "text"}
          placeholder={facet?.unit}
          style={field}
        />
      )}

      {!isOutput && (
        <div style={{ display: "flex", gap: 4 }}>
          <button onClick={() => setMode("override")} style={segBtn(mode === "override")}>
            Override
          </button>
          <button onClick={() => setMode("default")} style={segBtn(mode === "default")}>
            Default
          </button>
        </div>
      )}

      {(isOutput || mode === "override") && (
        <label style={{ display: "flex", alignItems: "center", gap: 6, color: "#8892a0", fontSize: 11 }}>
          duration
          <select
            value={duration}
            onChange={(e) => setDuration(e.target.value)}
            style={{ ...field, flex: 1 }}
          >
            <option value="10">10 sec</option>
            <option value="30">30 sec</option>
            <option value="60">1 min</option>
            <option value="300">5 min</option>
            <option value="1200">20 min</option>
            <option value="3600">1 hr</option>
            <option value="7200">2 hr</option>
            <option value="86400">24 hr</option>
            <option value="0">permanent</option>
          </select>
        </label>
      )}

      <div style={{ display: "flex", gap: 6, marginTop: 2 }}>
        {overridden && (
          <button
            onClick={onClear}
            style={{ ...segBtn(false), flex: "0 0 auto", color: "#c98a8a" }}
            title="Clear override"
          >
            clear
          </button>
        )}
        <button onClick={onClose} style={{ ...segBtn(false), marginLeft: "auto", flex: "0 0 auto" }}>
          cancel
        </button>
        <button
          onClick={apply}
          style={{ ...segBtn(true), flex: "0 0 auto", borderColor: "#3b5388" }}
        >
          set
        </button>
      </div>
    </div>
  );
}

// Same OVR badge the wiresheet node rows use.
function OvrBadge() {
  return (
    <span
      title="overridden"
      style={{
        fontSize: 9,
        padding: "0 4px",
        background: "#f59e0b",
        color: "#0f1115",
        borderRadius: 2,
        fontWeight: 600,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
      }}
    >
      OVR
    </span>
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
