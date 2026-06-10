import { useEffect, useState, type FormEvent } from "react";
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

import type { DashboardSummary } from "@/api/types";
import {
  DashboardForm,
  type DashboardFormValues,
} from "@/features/dashboards/DashboardForm";
import { useUpdateDashboard } from "@/features/dashboards/useDashboardMutations";

// Edit a dashboard's name + icon + accent — the same fields and the same
// `DashboardForm` the create dialog uses, so the two flows can't drift. The
// slug is intentionally left untouched (renaming the name doesn't re-slug),
// so existing links stay valid. PATCHes only the changed appearance fields.
export function EditDashboardDialog({
  dashboard,
  onClose,
}: {
  dashboard: DashboardSummary | null;
  onClose: () => void;
}) {
  const update = useUpdateDashboard();
  const [values, setValues] = useState<DashboardFormValues>({
    name: "",
    icon: "Activity",
    accent: "152 76% 44%",
  });

  // Seed the form when a dashboard is selected for editing. Keyed on the id
  // so opening a different row reloads its current appearance.
  useEffect(() => {
    if (dashboard) {
      setValues({
        name: dashboard.name,
        icon: dashboard.icon,
        accent: dashboard.accent,
      });
    }
  }, [dashboard]);

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!dashboard || !values.name.trim()) return;
    update.mutate(
      {
        slug: dashboard.slug,
        patch: {
          name: values.name.trim(),
          icon: values.icon,
          accent: values.accent,
        },
      },
      { onSuccess: onClose },
    );
  }

  return (
    <Dialog open={dashboard !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="glass">
        <DialogHeader>
          <DialogTitle>Edit dashboard</DialogTitle>
          <DialogDescription>
            Update the name, icon, and accent.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={onSubmit}>
          <DashboardForm
            values={values}
            onChange={setValues}
            nameId="edit-dashboard-name"
          />
          {update.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {update.error instanceof StarterError &&
              update.error.status === 409
                ? "That slug is already taken."
                : "Couldn't save the dashboard."}
            </p>
          ) : null}
          <DialogFooter>
            <Button variant="outline" type="button" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={update.isPending || !values.name.trim()}
            >
              {update.isPending ? "Saving…" : "Save changes"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
