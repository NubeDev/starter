// The `tree` widget — the folder hierarchy as an expandable tree with a
// component-count badge per folder, for a structural overview of the graph.
// Selection is shared with the host (sync): click selects, double-click drills
// in. Built from the store's parent relationships; counts use childrenCount so a
// folder shows its size even before its children are loaded.

import { useEffect, useMemo, useState } from "react";
import { useStructural } from "../lib/store";
import { ChevronRight, ChevronDown, Folder, FolderOpen, Dot } from "lucide-react";
import type { Component } from "../lib/engine-types";

export interface TreeWidgetProps {
  currentParentUid: number;
  selectedUids: number[];
  onSelect?: (uids: number[]) => void;
  onDrillIn?: (uid: number) => void;
}

export function TreeWidget({ currentParentUid, selectedUids, onSelect, onDrillIn }: TreeWidgetProps) {
  const components = useStructural((s) => s.components);
  // Everything collapsed by default; the root folder starts open so the first
  // level is visible. Reset when the current folder changes (navigation).
  const [expanded, setExpanded] = useState<Set<number>>(() => new Set([currentParentUid]));
  useEffect(() => setExpanded(new Set([currentParentUid])), [currentParentUid]);

  const childrenByParent = useMemo(() => {
    const m = new Map<number, Component[]>();
    for (const c of components.values()) {
      const arr = m.get(c.parent);
      if (arr) arr.push(c);
      else m.set(c.parent, [c]);
    }
    // Folders (components with children) first, then leaves — each alphabetical.
    const isFolder = (c: Component) => (c.childrenCount ?? 0) > 0 || (m.get(c.uid)?.length ?? 0) > 0;
    for (const arr of m.values())
      arr.sort((a, b) => (isFolder(b) ? 1 : 0) - (isFolder(a) ? 1 : 0) || a.name.localeCompare(b.name));
    return m;
  }, [components]);

  // Total components contained under a folder (recursively, over loaded data).
  const totalUnder = useMemo(() => {
    const memo = new Map<number, number>();
    const count = (uid: number): number => {
      if (memo.has(uid)) return memo.get(uid)!;
      let n = 0;
      for (const c of childrenByParent.get(uid) ?? []) n += 1 + count(c.uid);
      memo.set(uid, n);
      return n;
    };
    return count;
  }, [childrenByParent]);

  const sel = new Set(selectedUids);
  const toggle = (uid: number) =>
    setExpanded((s) => {
      const n = new Set(s);
      n.has(uid) ? n.delete(uid) : n.add(uid);
      return n;
    });

  const renderRow = (
    uid: number,
    name: string,
    depth: number,
    isFolder: boolean,
    count: number,
    selectable: boolean,
  ): React.ReactNode => {
    const open = isFolder && expanded.has(uid);
    const kids = childrenByParent.get(uid) ?? [];
    return (
      <div key={uid}>
        <div
          onClick={selectable ? () => onSelect?.([uid]) : isFolder ? () => toggle(uid) : undefined}
          onDoubleClick={selectable ? () => onDrillIn?.(uid) : undefined}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            padding: "3px 6px",
            paddingLeft: 6 + depth * 14,
            cursor: isFolder || selectable ? "pointer" : "default",
            background: sel.has(uid) ? "#2b3550" : "transparent",
            fontSize: 12,
            whiteSpace: "nowrap",
          }}
        >
          {isFolder ? (
            <button onClick={(e) => { e.stopPropagation(); toggle(uid); }} style={chevBtn} title={open ? "Collapse" : "Expand"}>
              {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            </button>
          ) : (
            <span style={{ width: 15, flexShrink: 0 }} />
          )}
          {isFolder ? (
            open ? <FolderOpen size={13} color="#5a86c7" /> : <Folder size={13} color="#5a86c7" />
          ) : (
            <Dot size={13} color="#5a6172" />
          )}
          <span style={{ color: "#e6e8eb" }}>{name}</span>
          {isFolder && <span style={countBadge}>{count}</span>}
        </div>
        {open && kids.map((k) => {
          const kf = (k.childrenCount ?? 0) > 0 || (childrenByParent.get(k.uid)?.length ?? 0) > 0;
          return renderRow(k.uid, k.name, depth + 1, kf, totalUnder(k.uid) || (k.childrenCount ?? 0), true);
        })}
      </div>
    );
  };

  const rootComp = components.get(currentParentUid);
  const rootName = rootComp?.name ?? "root";
  const rootCount = totalUnder(currentParentUid) || (rootComp?.childrenCount ?? 0);
  const hasAny = (childrenByParent.get(currentParentUid)?.length ?? 0) > 0;

  return (
    <div style={{ height: "100%", overflow: "auto", userSelect: "none" }}>
      {!hasAny ? (
        <div style={{ padding: 12, color: "#5a6172", fontSize: 12 }}>no components in this folder</div>
      ) : (
        renderRow(currentParentUid, rootName, 0, true, rootCount, false)
      )}
    </div>
  );
}

const chevBtn: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  width: 15,
  height: 15,
  border: "none",
  background: "transparent",
  color: "#9aa3b2",
  cursor: "pointer",
  padding: 0,
  flexShrink: 0,
};
const countBadge: React.CSSProperties = {
  marginLeft: 6,
  fontSize: 9,
  color: "#9aa3b2",
  background: "#23272f",
  borderRadius: 8,
  padding: "0 6px",
  lineHeight: "14px",
};
