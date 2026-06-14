import { Tag as TagIcon } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@nube/starter-ui-kit/components/popover";

import { TagEditor } from "@/features/tags/TagEditor";
import { useTags } from "@/features/tags/useTags";

// The dashboard's tags, edited in a popover off the toolbar. Tags reuse the
// generic tagging system (`kind = "dashboard"`), so this is just the shared
// `TagEditor` mounted for this dashboard's id. The trigger shows the current
// tag count so the affordance reads as "n tags" at a glance.
export function DashboardTagsButton({ dashboardId }: { dashboardId: string }) {
  const { data: tags } = useTags("dashboard", dashboardId);
  const count = tags?.length ?? 0;

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="gap-2"
          title="Tag this dashboard"
        >
          <TagIcon className="size-4" />
          {count > 0 ? <span>{count}</span> : <span>Tags</span>}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <div className="flex flex-col gap-2">
          <span className="text-xs font-medium text-muted-foreground">
            Dashboard tags
          </span>
          <TagEditor kind="dashboard" id={dashboardId} />
        </div>
      </PopoverContent>
    </Popover>
  );
}
