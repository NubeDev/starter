import { useState } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { TimeRange } from "@/store/time";
import { fromDatetimeLocal, toDatetimeLocal } from "@/features/time/datetimeLocal";

// The "absolute" tab of the picker: two `datetime-local` fields seeded from
// the current range (resolved to concrete instants), applied as UTC ISO
// bounds. Kept separate from the picker shell so the picker stays a thin
// layout and this owns only the absolute-form state.
export function AbsoluteRangeForm({
  range,
  now,
  onApply,
}: {
  range: TimeRange;
  now: Date;
  onApply: (range: TimeRange) => void;
}) {
  const [from, setFrom] = useState(() => toDatetimeLocal(range.from, now));
  const [to, setTo] = useState(() => toDatetimeLocal(range.to, now));

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1">
        <Label htmlFor="time-from" className="text-xs">
          From
        </Label>
        <Input
          id="time-from"
          type="datetime-local"
          value={from}
          onChange={(e) => setFrom(e.target.value)}
        />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="time-to" className="text-xs">
          To
        </Label>
        <Input
          id="time-to"
          type="datetime-local"
          value={to}
          onChange={(e) => setTo(e.target.value)}
        />
      </div>
      <Button
        size="sm"
        // An empty field would resolve to Invalid Date; gate apply on both.
        disabled={!from || !to}
        onClick={() =>
          onApply({ from: fromDatetimeLocal(from), to: fromDatetimeLocal(to) })
        }
      >
        Apply time range
      </Button>
    </div>
  );
}
