import { RefreshCw } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { useTimeStore } from "@/store/time";
import { REFRESH_OPTIONS } from "@/features/time/quickRanges";

// The refresh control: a manual-refresh button (bumps the tick now) plus an
// interval dropdown that arms the auto-refresh loop (`useAutoRefresh` reads
// `refresh` from the same store). Keeping both here lets a user one-shot
// refresh without committing to a polling interval.
export function RefreshControl() {
  const refresh = useTimeStore((s) => s.refresh);
  const setRefresh = useTimeStore((s) => s.setRefresh);
  const bump = useTimeStore((s) => s.bump);

  return (
    <div className="flex items-center gap-1">
      <Button
        variant="outline"
        size="sm"
        className="gap-2"
        onClick={() => bump()}
        aria-label="Refresh now"
        title="Refresh now"
      >
        <RefreshCw className="size-4" />
      </Button>
      <Select
        value={String(refresh)}
        onValueChange={(v) => setRefresh(Number(v))}
      >
        <SelectTrigger className="w-20" aria-label="Auto-refresh interval">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {REFRESH_OPTIONS.map((o) => (
            <SelectItem key={o.secs} value={String(o.secs)}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
