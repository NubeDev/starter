import { useState } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { TimeRange } from "@/store/time";
import { resolveBound } from "@/store/time";

// The "relative" tab: free-text `from`/`to` tokens (`now-6h`, `now/d`, `now`)
// validated against the resolver before apply, so a typo surfaces inline
// instead of silently breaking every panel query.
export function RelativeRangeForm({
  range,
  now,
  onApply,
}: {
  range: TimeRange;
  now: Date;
  onApply: (range: TimeRange) => void;
}) {
  const [from, setFrom] = useState(range.from);
  const [to, setTo] = useState(range.to);

  const error = validate(from, to, now);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1">
        <Label htmlFor="rel-from" className="text-xs">
          From
        </Label>
        <Input
          id="rel-from"
          value={from}
          onChange={(e) => setFrom(e.target.value)}
          placeholder="now-6h"
        />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="rel-to" className="text-xs">
          To
        </Label>
        <Input
          id="rel-to"
          value={to}
          onChange={(e) => setTo(e.target.value)}
          placeholder="now"
        />
      </div>
      {error ? <p className="text-xs text-destructive">{error}</p> : null}
      <Button
        size="sm"
        disabled={Boolean(error)}
        onClick={() => onApply({ from, to })}
      >
        Apply time range
      </Button>
    </div>
  );
}

// Both bounds must parse, and the window must be non-empty (from < to once
// resolved) — an inverted range yields no rows from every panel.
function validate(from: string, to: string, now: Date): string | null {
  let fromT: number;
  let toT: number;
  try {
    fromT = resolveBound(from, now).getTime();
  } catch {
    return `Invalid "from": ${from}`;
  }
  try {
    toT = resolveBound(to, now).getTime();
  } catch {
    return `Invalid "to": ${to}`;
  }
  if (fromT >= toT) return "\"From\" must be before \"to\".";
  return null;
}
