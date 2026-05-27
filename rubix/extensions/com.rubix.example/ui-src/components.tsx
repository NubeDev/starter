import * as React from "react";
import type { CustomerRow } from "./types";
import { evaluateCustomerQuality } from "./quality";

export function ContribRow({
  label,
  items,
}: {
  label: string;
  items: ReadonlyArray<string>;
}): React.ReactElement {
  return (
    <>
      <dt style={{ opacity: 0.7 }}>{label}</dt>
      <dd style={{ margin: 0 }}>
        {items.length === 0 ? (
          <span style={{ opacity: 0.5 }}>—</span>
        ) : (
          items.map((id, i) => (
            <React.Fragment key={id + i}>
              {i > 0 ? ", " : ""}
              <code>{id}</code>
            </React.Fragment>
          ))
        )}
      </dd>
    </>
  );
}

export function Card({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <section
      style={{
        padding: "0.9rem 1rem",
        borderRadius: "0.6rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem",
      }}
    >
      <header>
        <h4 style={{ margin: 0, fontSize: "0.95rem" }}>{title}</h4>
        {subtitle ? (
          <small style={{ opacity: 0.65 }}>{subtitle}</small>
        ) : null}
      </header>
      {children}
    </section>
  );
}

// Horizontal bar chart — pure SVG, no chart library. Mirrors the
// `customers_by_country` template's `{ country, customer_count }`
// projection.
export function CountryBarChart({
  rows,
}: {
  rows: ReadonlyArray<CustomerRow>;
}): React.ReactElement | null {
  const buckets = React.useMemo(() => {
    const m = new Map<string, number>();
    for (const r of rows) {
      const q = evaluateCustomerQuality(r);
      if (q.outcome !== "ok" && q.quality === "MissingCountry") continue;
      const c = (r.country || "").trim() || "(unknown)";
      m.set(c, (m.get(c) || 0) + 1);
    }
    return [...m.entries()]
      .map(([country, count]) => ({ country, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);
  }, [rows]);

  if (buckets.length === 0) return null;
  const max = Math.max(...buckets.map((b) => b.count));
  const labelW = 160;
  const barH = 18;
  const gap = 6;
  const w = 360;
  const height = buckets.length * (barH + gap);

  return (
    <svg
      width="100%"
      viewBox={`0 0 ${labelW + w + 40} ${height}`}
      role="img"
      aria-label="Customers by country (top 10)"
      style={{ display: "block" }}
    >
      {buckets.map((b, i) => {
        const y = i * (barH + gap);
        const bw = Math.max(1, Math.round((b.count / max) * w));
        return (
          <React.Fragment key={b.country}>
            <text
              x={labelW - 8}
              y={y + barH * 0.72}
              textAnchor="end"
              fontSize={12}
              fill="currentColor"
              opacity={0.85}
            >
              {b.country}
            </text>
            <rect
              x={labelW}
              y={y}
              width={bw}
              height={barH}
              rx={3}
              fill="var(--color-accent, #4f46e5)"
              opacity={0.85}
            />
            <text
              x={labelW + bw + 6}
              y={y + barH * 0.72}
              fontSize={12}
              fill="currentColor"
              opacity={0.85}
            >
              {b.count}
            </text>
          </React.Fragment>
        );
      })}
    </svg>
  );
}
