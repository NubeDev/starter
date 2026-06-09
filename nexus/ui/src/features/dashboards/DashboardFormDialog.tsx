import { useState, type FormEvent } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { useStarterClient } from "@nube/starter-client-react";
import { StarterError } from "@nube/starter-client-ts";
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

import { createDashboard } from "@/api/dashboards/create";
import type { DashboardSummary } from "@/api/types";
import { DASHBOARDS_KEY } from "@/features/dashboards/useDashboards";

// Slugs are lower-kebab; derive one from the name so the user types once.
function slugify(name: string): string {
  return name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

// Create-dashboard dialog. On success it invalidates the sidebar list and
// navigates to the new dashboard's slug. A 409 (slug taken) surfaces as a
// field-level message rather than a generic failure.
export function DashboardFormDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  const create = useMutation<DashboardSummary, Error, string>({
    mutationFn: (n) => createDashboard(client, { name: n, slug: slugify(n) }),
    onSuccess: (summary) => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
      onOpenChange(false);
      setName("");
      navigate(`/d/${summary.slug}`);
    },
    onError: (err) => {
      setError(
        err instanceof StarterError && err.status === 409
          ? "A dashboard with that name already exists."
          : "Couldn't create the dashboard.",
      );
    },
  });

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    if (name.trim()) create.mutate(name.trim());
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass">
        <DialogHeader>
          <DialogTitle>New dashboard</DialogTitle>
          <DialogDescription>Give it a name to get started.</DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={onSubmit}>
          <div className="space-y-2">
            <Label htmlFor="dashboard-name">Name</Label>
            <Input
              id="dashboard-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Cold chain"
              autoFocus
              required
            />
          </div>
          {error ? (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={create.isPending || !name.trim()}>
              {create.isPending ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
