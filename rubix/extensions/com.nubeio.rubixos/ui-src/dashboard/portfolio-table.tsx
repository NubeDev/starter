// Compact, sortable, searchable site list. Renders ~100 rows of
// plain DOM happily — only sparklines remain (cheap SVG). Used
// when site count exceeds the comfortable tile threshold.

import * as React from "react";

import { Sparkline } from "../sparkline";
import { fmtBig } from "./helpers";

type SortKey = "rank" | "name" | "region" | "last" | "total" | "share";
type SortDir = "asc" | "desc";

export interface PortfolioRow {
  host_uuid: string;
  name: string;
  region: string;
  total: number;
  last: number | null;
  share: number;
  spark: ReadonlyArray<number | null>;
}

export function PortfolioTable({
  rows, unit, selectedHosts, onToggleHost, query, setQuery,
}: {
  rows: ReadonlyArray<PortfolioRow>;
  unit: string | null;
  selectedHosts: ReadonlyArray<string>;
  onToggleHost: (uuid: string) => void;
  query: string;
  setQuery: (s: string) => void;
}): React.ReactElement {
  const [sortKey, setSortKey] = React.useState<SortKey>("total");
  const [sortDir, setSortDir] = React.useState<SortDir>("desc");

  const filtered = React.useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(
      (r) =>
        r.name.toLowerCase().includes(q) ||
        r.region.toLowerCase().includes(q),
    );
  }, [rows, query]);

  const sorted = React.useMemo(() => {
    const arr = filtered.slice();
    const dir = sortDir === "asc" ? 1 : -1;
    arr.sort((a, b) => {
      const get = (r: PortfolioRow): number | string => {
        switch (sortKey) {
          case "name":   return r.name;
          case "region": return r.region;
          case "last":   return r.last ?? -Infinity;
          case "share":  return r.share;
          case "total":  return r.total;
          case "rank":   return r.total;
        }
      };
      const av = get(a);
      const bv = get(b);
      if (typeof av === "string" && typeof bv === "string") return dir * av.localeCompare(bv);
      return dir * ((av as number) - (bv as number));
    });
    return arr;
  }, [filtered, sortKey, sortDir]);

  const sel = new Set(selectedHosts);
  const Header = ({ k, label, align = "left" }: { k: SortKey; label: string; align?: "left" | "right" }) => {
    const active = sortKey === k;
    const arrow = active ? (sortDir === "asc" ? " ↑" : " ↓") : "";
    return (
      <th
        scope="col"
        className={
          "py-1.5 px-2 font-medium select-none cursor-pointer " +
          (align === "right" ? "text-right " : "") +
          (active ? "text-foreground" : "")
        }
        onClick={() => {
          if (sortKey === k) setSortDir(sortDir === "asc" ? "desc" : "asc");
          else { setSortKey(k); setSortDir(k === "name" || k === "region" ? "asc" : "desc"); }
        }}
        aria-sort={active ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
      >
        {label}{arrow}
      </th>
    );
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter sites…"
          aria-label="Filter sites"
          className={
            "ext-glass px-3 py-1.5 text-sm rounded-md w-full max-w-xs " +
            "bg-transparent focus:outline-none focus-visible:ring-2 focus-visible:ring-primary"
          }
        />
        <span className="ext-eyebrow tabular-nums">
          {sorted.length} / {rows.length}
        </span>
      </div>
      <div className="overflow-x-auto max-h-[60vh]">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-card/80 backdrop-blur text-xs text-muted-foreground">
            <tr className="text-left">
              <th className="py-1.5 px-2 w-8 text-right text-muted-foreground/70">#</th>
              <Header k="name"   label="Site" />
              <Header k="region" label="Region" />
              <th className="py-1.5 px-2 w-32 font-medium text-muted-foreground/70">Trend</th>
              <Header k="last"   label={`Latest${unit ? ` (${unit})` : ""}`} align="right" />
              <Header k="total"  label={`Σ${unit ? ` (${unit})` : ""}`}      align="right" />
              <Header k="share"  label="Share"  align="right" />
            </tr>
          </thead>
          <tbody>
            {sorted.map((r, i) => {
              const on = sel.has(r.host_uuid);
              return (
                <tr
                  key={r.host_uuid}
                  onClick={() => onToggleHost(r.host_uuid)}
                  className={
                    "border-t border-border/40 cursor-pointer transition-colors " +
                    (on ? "bg-primary/5" : "hover:bg-accent/30") +
                    " focus-within:bg-accent/30"
                  }
                  aria-pressed={on}
                >
                  <td className="py-1.5 px-2 text-right tabular-nums text-muted-foreground">{i + 1}</td>
                  <td className="py-1.5 px-2 min-w-0">
                    <span
                      className={
                        "inline-block h-1.5 w-1.5 rounded-full mr-2 align-middle " +
                        (on ? "bg-primary" : "bg-muted-foreground/30")
                      }
                      aria-hidden="true"
                    />
                    <span className="font-medium">{r.name}</span>
                  </td>
                  <td className="py-1.5 px-2 text-muted-foreground">{r.region}</td>
                  <td className="py-1.5 px-2 text-primary">
                    <Sparkline values={r.spark} width={110} height={22} color="currentColor" />
                  </td>
                  <td className="py-1.5 px-2 text-right tabular-nums ext-num">
                    {r.last !== null ? fmtBig(r.last) : "—"}
                  </td>
                  <td className="py-1.5 px-2 text-right tabular-nums ext-num">
                    {fmtBig(r.total)}
                  </td>
                  <td className="py-1.5 px-2 text-right tabular-nums">
                    <span className="text-muted-foreground">{(r.share * 100).toFixed(1)}%</span>
                  </td>
                </tr>
              );
            })}
            {sorted.length === 0 ? (
              <tr><td colSpan={7} className="py-4 px-2 text-muted-foreground italic">No sites match.</td></tr>
            ) : null}
          </tbody>
        </table>
      </div>
    </div>
  );
}
