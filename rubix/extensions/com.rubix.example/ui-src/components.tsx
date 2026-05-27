// `ui/components.tsx` — reusable presentational components (shadcn/ui).

import * as React from "react";
import type { CountryBucket } from "./types";

export function ContribRow({
  label,
  items,
}: {
  label: string;
  items: ReadonlyArray<string>;
}): React.ReactElement {
  return (
    <div className="flex items-baseline gap-3 text-sm">
      <span className="text-muted-foreground shrink-0">{label}</span>
      <span className="flex flex-wrap gap-1">
        {items.length === 0 ? (
          <span className="text-muted-foreground/50">—</span>
        ) : (
          items.map((id, i) => (
            <code key={id + i} className="rounded bg-muted px-1.5 py-0.5 text-xs font-mono">
              {id}
            </code>
          ))
        )}
      </span>
    </div>
  );
}

// Horizontal bar chart — pure SVG. Takes server-aggregated buckets
// straight from `com.rubix.example.customers_by_country`.
export function CountryBarChart({
  buckets,
}: {
  buckets: ReadonlyArray<CountryBucket>;
}): React.ReactElement | null {
  if (buckets.length === 0) return null;
  const max = Math.max(...buckets.map((b) => b.customer_count));
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
      className="block"
    >
      {buckets.map((b, i) => {
        const y = i * (barH + gap);
        const bw = Math.max(1, Math.round((b.customer_count / max) * w));
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
              className="fill-primary opacity-85"
            />
            <text
              x={labelW + bw + 6}
              y={y + barH * 0.72}
              fontSize={12}
              fill="currentColor"
              opacity={0.85}
            >
              {b.customer_count}
            </text>
          </React.Fragment>
        );
      })}
    </svg>
  );
}
