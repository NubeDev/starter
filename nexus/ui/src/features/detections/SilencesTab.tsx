import { useState, type FormEvent } from "react";
import { BellOff, Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import {
  useSilenceMutations,
  useSilences,
} from "@/features/detections/useNotify";
import { useDateTime } from "@/datetime";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Silence windows that mute notifications for a span (the detection still runs
// and records findings; it just doesn't page). A null detection_id mutes all
// detections; this form creates a tenant-wide silence. Times are entered as
// local datetime and sent as ISO strings.
export function SilencesTab() {
  const { data, isPending, isError, error } = useSilences();
  const { create, remove } = useSilenceMutations();
  const { dateTime } = useDateTime();
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState({ starts_at: "", ends_at: "", reason: "" });

  function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    create.mutate(
      {
        starts_at: new Date(form.starts_at).toISOString(),
        ends_at: new Date(form.ends_at).toISOString(),
        reason: form.reason.trim() || null,
      },
      {
        onSuccess: () => {
          setOpen(false);
          setForm({ starts_at: "", ends_at: "", reason: "" });
        },
      },
    );
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex justify-end">
        <Button size="sm" className="gap-2" onClick={() => setOpen(true)}>
          <Plus className="size-4" />
          New silence
        </Button>
      </div>

      {isPending ? (
        <Loading label="Loading silences…" />
      ) : isError ? (
        <ErrorState message={error instanceof Error ? error.message : undefined} />
      ) : data.length === 0 ? (
        <Empty title="No silences" description="Mute notifications for a window." />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((s) => (
            <li
              key={s.id}
              className="glass flex items-center gap-3 rounded-lg px-4 py-3"
            >
              <span className="grid size-9 place-items-center rounded-lg bg-muted text-muted-foreground">
                <BellOff className="size-4" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="tabular truncate text-sm text-foreground">
                  {dateTime(s.starts_at)} → {dateTime(s.ends_at)}
                </p>
                <p className="truncate text-xs text-muted-foreground">
                  {s.reason ?? "No reason given"}
                  {s.detection_id ? " · one detection" : " · all detections"}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                aria-label="Delete silence"
                disabled={remove.isPending}
                onClick={() => remove.mutate(s.id)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="size-4" />
              </Button>
            </li>
          ))}
        </ul>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="glass max-w-md">
          <DialogHeader>
            <DialogTitle>New silence</DialogTitle>
          </DialogHeader>
          <form className="space-y-3" onSubmit={submit}>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label htmlFor="sil-start">Starts</Label>
                <Input
                  id="sil-start"
                  type="datetime-local"
                  value={form.starts_at}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, starts_at: e.target.value }))
                  }
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="sil-end">Ends</Label>
                <Input
                  id="sil-end"
                  type="datetime-local"
                  value={form.ends_at}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, ends_at: e.target.value }))
                  }
                  required
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sil-reason">Reason</Label>
              <Input
                id="sil-reason"
                value={form.reason}
                onChange={(e) => setForm((f) => ({ ...f, reason: e.target.value }))}
                placeholder="Planned maintenance"
              />
            </div>
            <DialogFooter>
              <Button
                type="submit"
                disabled={create.isPending || !form.starts_at || !form.ends_at}
              >
                {create.isPending ? "Creating…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
