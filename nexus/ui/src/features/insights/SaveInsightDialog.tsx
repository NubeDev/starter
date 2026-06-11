import { useState, type FormEvent } from "react";
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

import type { CreateInsightRequest } from "@/api/types";
import { useCreateInsight } from "@/features/insights/useInsightMutations";

// Saves the Workbench's current transform script as a named, reusable insight.
// The backend compile-checks the script and returns a 400 with a message when
// it doesn't compile; that message is surfaced inline (role="alert") so the
// user can fix it without losing the dialog. On success the create invalidates
// the insights list (so the list page and the "Load saved insight" dropdown both
// refresh) and the optional `onSaved` callback fires (the Workbench uses it to
// return to the list).
export function SaveInsightDialog({
  open,
  onOpenChange,
  script,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  script: string;
  onSaved?: (name: string) => void;
}) {
  const create = useCreateInsight();
  const [name, setName] = useState("");

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const body: CreateInsightRequest = { name: name.trim(), script };
    create.mutate(body, {
      onSuccess: () => {
        onSaved?.(name.trim());
        onOpenChange(false);
        create.reset();
        setName("");
      },
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>Save as insight</DialogTitle>
          <DialogDescription>
            Save the current transform as a reusable insight. It will appear in
            your Insights list to reuse on panels and in Explore.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="save-insight-name">Name</Label>
            <Input
              id="save-insight-name"
              value={name}
              onChange={(e) => {
                create.reset();
                setName(e.target.value);
              }}
              autoComplete="off"
              placeholder="Hourly z-score outliers"
              required
            />
          </div>
          <div className="rounded-md border border-border/60 bg-background/40 p-2">
            <pre className="scrollbar-thin max-h-24 overflow-auto font-mono text-xs text-muted-foreground">
              {script.trim() || "(empty script)"}
            </pre>
          </div>
          {create.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {create.error instanceof Error
                ? create.error.message
                : "Couldn't save the insight."}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              type="submit"
              disabled={create.isPending || !script.trim()}
            >
              {create.isPending ? "Saving…" : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
