import { useState } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type {
  CreateNavNodeRequest,
  NavNodeDetail,
  StaticRoute,
} from "@/api/types";
import { useDashboards } from "@/features/dashboards/useDashboards";
import { ROUTE_META, STATIC_ROUTES } from "@/features/nav/routeMeta";

type TargetKind = "group" | "dashboard" | "route";

// Add / edit a nav node (WS-13 §4/§7). Pick a target — a `group` header, a
// reusable `dashboard` mount (+ context values), or a static `route` from the
// closed allow-list — then save. Emits a CreateNavNodeRequest-shaped payload;
// the caller decides create vs update and supplies parent_id/sort_order.
export function NavNodeFormDialog({
  open,
  initial,
  onSubmit,
  onClose,
}: {
  open: boolean;
  initial?: NavNodeDetail;
  onSubmit: (payload: Pick<CreateNavNodeRequest, "title" | "target" | "context">) => void;
  onClose: () => void;
}) {
  const { data: dashboards } = useDashboards();

  const [title, setTitle] = useState(initial?.title ?? "");
  const [targetKind, setTargetKind] = useState<TargetKind>(
    initial?.target.kind ?? "group",
  );
  const [dashboardId, setDashboardId] = useState(
    initial?.target.kind === "dashboard" ? initial.target.dashboardId : "",
  );
  const [route, setRoute] = useState<StaticRoute>(
    initial?.target.kind === "route" ? initial.target.route : "dashboards",
  );
  // Context values authored as `key=value` lines (one per line), only for a
  // dashboard mount. Seeded from the node's existing context.values.
  const [valuesText, setValuesText] = useState(
    initial?.context?.values
      ? Object.entries(initial.context.values)
          .map(([k, v]) => `${k}=${String(v)}`)
          .join("\n")
      : "",
  );

  const titleValid = title.trim().length > 0;
  const dashboardValid = targetKind !== "dashboard" || dashboardId.length > 0;

  function buildTarget(): CreateNavNodeRequest["target"] {
    if (targetKind === "dashboard") return { kind: "dashboard", dashboardId };
    if (targetKind === "route") return { kind: "route", route };
    return { kind: "group" };
  }

  function buildContext(): CreateNavNodeRequest["context"] {
    if (targetKind !== "dashboard") return undefined;
    const values: Record<string, string> = {};
    for (const line of valuesText.split("\n")) {
      const eq = line.indexOf("=");
      if (eq === -1) continue;
      const key = line.slice(0, eq).trim();
      const value = line.slice(eq + 1).trim();
      if (key) values[key] = value;
    }
    return Object.keys(values).length > 0 ? { values } : undefined;
  }

  function submit() {
    if (!titleValid || !dashboardValid) return;
    onSubmit({ title: title.trim(), target: buildTarget(), context: buildContext() });
  }

  return (
    <Dialog open={open} onOpenChange={(o) => (!o ? onClose() : undefined)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{initial ? "Edit node" : "Add node"}</DialogTitle>
          <DialogDescription>
            A group organises; a dashboard mount reuses a page with its own
            context; a route links a built-in page.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="nn-title">Title</Label>
            <Input
              id="nn-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Building-1"
              aria-invalid={!titleValid && title.length > 0}
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="nn-kind">Type</Label>
            <Select
              value={targetKind}
              onValueChange={(v) => setTargetKind(v as TargetKind)}
            >
              <SelectTrigger id="nn-kind">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="group">Group (header)</SelectItem>
                <SelectItem value="dashboard">Dashboard mount</SelectItem>
                <SelectItem value="route">Static page</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {targetKind === "dashboard" ? (
            <>
              <div className="space-y-1.5">
                <Label htmlFor="nn-dash">Page</Label>
                <Select value={dashboardId} onValueChange={setDashboardId}>
                  <SelectTrigger id="nn-dash">
                    <SelectValue placeholder="Choose a dashboard…" />
                  </SelectTrigger>
                  <SelectContent>
                    {(dashboards ?? []).map((d) => (
                      <SelectItem key={d.id} value={d.id}>
                        {d.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="nn-ctx">Context values</Label>
                <textarea
                  id="nn-ctx"
                  value={valuesText}
                  onChange={(e) => setValuesText(e.target.value)}
                  placeholder={"building=b1\nsite=north"}
                  rows={3}
                  className="w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm"
                />
                <p className="text-xs text-muted-foreground">
                  One <code>key=value</code> per line. Read by a{" "}
                  <code>context</code>/<code>values</code> variable on the page.
                </p>
              </div>
            </>
          ) : null}

          {targetKind === "route" ? (
            <div className="space-y-1.5">
              <Label htmlFor="nn-route">Page</Label>
              <Select
                value={route}
                onValueChange={(v) => setRoute(v as StaticRoute)}
              >
                <SelectTrigger id="nn-route">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {STATIC_ROUTES.map((r) => (
                    <SelectItem key={r} value={r}>
                      {ROUTE_META[r].label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!titleValid || !dashboardValid}>
            {initial ? "Save" : "Add"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
