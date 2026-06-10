import { useState } from "react";
import { Clock } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@nube/starter-ui-kit/components/popover";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import type { TimeRange } from "@/store/time";
import { useTimeStore } from "@/store/time";
import { AbsoluteRangeForm } from "@/features/time/AbsoluteRangeForm";
import { RelativeRangeForm } from "@/features/time/RelativeRangeForm";
import { QUICK_RANGES, rangeLabel } from "@/features/time/quickRanges";

// The global time-range picker: a popover trigger labelled with the active
// range, holding a quick-range list plus absolute/relative tabs. Applying any
// of them writes the range to the time store (which freezes a fresh instant),
// re-running every time-macro panel. Closes on apply so the toolbar reads as
// a single committed selection.
export function TimeRangePicker() {
  const range = useTimeStore((s) => s.range);
  const now = useTimeStore((s) => s.now);
  const setRange = useTimeStore((s) => s.setRange);
  const [open, setOpen] = useState(false);

  const apply = (next: TimeRange) => {
    setRange(next);
    setOpen(false);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Clock className="size-4" />
          {rangeLabel(range)}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <div className="flex flex-col gap-1">
          <span className="text-xs font-medium text-muted-foreground">
            Quick ranges
          </span>
          <div className="grid grid-cols-2 gap-1">
            {QUICK_RANGES.map((q) => (
              <Button
                key={q.label}
                variant="ghost"
                size="sm"
                className="justify-start"
                onClick={() => apply(q.range)}
              >
                {q.label}
              </Button>
            ))}
          </div>
        </div>
        <Tabs defaultValue="relative">
          <TabsList className="w-full">
            <TabsTrigger value="relative">Relative</TabsTrigger>
            <TabsTrigger value="absolute">Absolute</TabsTrigger>
          </TabsList>
          <TabsContent value="relative">
            <RelativeRangeForm range={range} now={now} onApply={apply} />
          </TabsContent>
          <TabsContent value="absolute">
            <AbsoluteRangeForm range={range} now={now} onApply={apply} />
          </TabsContent>
        </Tabs>
      </PopoverContent>
    </Popover>
  );
}
