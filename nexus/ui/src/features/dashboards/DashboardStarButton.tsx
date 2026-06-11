import { Star } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { useStarredDashboards } from "@/features/me/useStarredDashboards";

// A star toggle for one dashboard. The star is per-user (stored in the caller's
// settings bag, not on the dashboard) so each user keeps their own favourites.
// Used in the dashboard toolbar and on the management list rows.
export function DashboardStarButton({
  dashboardId,
  size = "icon",
}: {
  dashboardId: string;
  // `icon` for compact toolbar/row use; `sm` when a label reads better.
  size?: "icon" | "sm";
}) {
  const { isStarred, toggle, isSaving } = useStarredDashboards();
  const starred = isStarred(dashboardId);
  const label = starred ? "Unstar dashboard" : "Star dashboard";

  return (
    <Button
      variant="ghost"
      size={size}
      onClick={(e) => {
        // On a list row the button often sits inside a link/clickable row;
        // toggling the star must not also navigate.
        e.preventDefault();
        e.stopPropagation();
        toggle(dashboardId);
      }}
      disabled={isSaving}
      aria-pressed={starred}
      aria-label={label}
      title={label}
    >
      <Star
        className={
          starred
            ? "fill-amber-400 text-amber-400"
            : "text-muted-foreground"
        }
      />
      {size === "sm" ? <span>{starred ? "Starred" : "Star"}</span> : null}
    </Button>
  );
}
