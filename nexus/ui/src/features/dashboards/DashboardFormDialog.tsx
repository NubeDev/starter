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

import { createDashboard } from "@/api/dashboards/create";
import type { DashboardSummary } from "@/api/types";
import {
  DashboardForm,
  type DashboardFormValues,
} from "@/features/dashboards/DashboardForm";
import { DEFAULT_ACCENT, DEFAULT_ICON } from "@/features/dashboards/appearance";
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
  const [values, setValues] = useState<DashboardFormValues>({
    name: "",
    icon: DEFAULT_ICON,
    accent: DEFAULT_ACCENT,
  });
  const [error, setError] = useState<string | null>(null);

  const create = useMutation<DashboardSummary, Error, DashboardFormValues>({
    mutationFn: (v) =>
      createDashboard(client, {
        name: v.name,
        slug: slugify(v.name),
        icon: v.icon,
        accent: v.accent,
      }),
    onSuccess: (summary) => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
      onOpenChange(false);
      setValues({ name: "", icon: DEFAULT_ICON, accent: DEFAULT_ACCENT });
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
    if (values.name.trim())
      create.mutate({ ...values, name: values.name.trim() });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass">
        <DialogHeader>
          <DialogTitle>New dashboard</DialogTitle>
          <DialogDescription>Give it a name to get started.</DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={onSubmit}>
          <DashboardForm values={values} onChange={setValues} />
          {error ? (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              type="submit"
              disabled={create.isPending || !values.name.trim()}
            >
              {create.isPending ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
