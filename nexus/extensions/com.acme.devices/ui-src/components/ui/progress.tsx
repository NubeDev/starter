// `progress.tsx` — minimal shadcn/ui-style determinate progress bar.
// Width-driven inner bar; `value` is 0–100. Kept dependency-free (no Radix)
// to stay inside the extension's slim bundle.

import * as React from "react";

import { cn } from "../../lib/utils";

export function Progress({
  value = 0,
  className,
}: {
  value?: number;
  className?: string;
}): React.ReactElement {
  const pct = Math.max(0, Math.min(100, value));
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct}
      className={cn(
        "relative h-2 w-full overflow-hidden rounded-full bg-secondary",
        className,
      )}
    >
      <div
        className="h-full rounded-full bg-primary transition-[width] duration-500 ease-out"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
