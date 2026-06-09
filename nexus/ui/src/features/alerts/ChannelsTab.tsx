import { useState, type FormEvent } from "react";
import { Plus, Send, Trash2 } from "lucide-react";
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
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import { useChannelMutations, useChannels } from "@/features/alerts/useAlerts";
import { parseFlowSection } from "@/features/flows/flowDraft";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Notification channels: list with kind + delete, and a create form whose
// `config` is opaque JSON (kind-specific — webhook url, email address…),
// validated client-side before send.
export function ChannelsTab() {
  const { data, isPending, isError, error } = useChannels();
  const { create, remove } = useChannelMutations();
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState({ name: "", kind: "webhook", config: "" });
  const [configError, setConfigError] = useState<string | null>(null);

  function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setConfigError(null);
    const parsed = parseFlowSection(form.config);
    if (!parsed.ok) {
      setConfigError(parsed.error);
      return;
    }
    create.mutate(
      { name: form.name.trim(), kind: form.kind.trim(), config: parsed.value },
      {
        onSuccess: () => {
          setOpen(false);
          setForm({ name: "", kind: "webhook", config: "" });
        },
      },
    );
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex justify-end">
        <Button size="sm" className="gap-2" onClick={() => setOpen(true)}>
          <Plus className="size-4" />
          New channel
        </Button>
      </div>

      {isPending ? (
        <Loading label="Loading channels…" />
      ) : isError ? (
        <ErrorState message={error instanceof Error ? error.message : undefined} />
      ) : data.length === 0 ? (
        <Empty title="No channels" description="Add a channel to notify on alerts." />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((c) => (
            <li
              key={c.id}
              className="glass flex items-center gap-3 rounded-lg px-4 py-3"
            >
              <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
                <Send className="size-4" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-foreground">{c.name}</p>
                <p className="text-xs text-muted-foreground">{c.kind}</p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                aria-label={`Delete ${c.name}`}
                disabled={remove.isPending}
                onClick={() => remove.mutate(c.id)}
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
            <DialogTitle>New channel</DialogTitle>
          </DialogHeader>
          <form className="space-y-3" onSubmit={submit}>
            <div className="space-y-1.5">
              <Label htmlFor="ch-name">Name</Label>
              <Input
                id="ch-name"
                value={form.name}
                onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="ch-kind">Kind</Label>
              <Input
                id="ch-kind"
                value={form.kind}
                onChange={(e) => setForm((f) => ({ ...f, kind: e.target.value }))}
                placeholder="webhook"
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="ch-config">Config (JSON)</Label>
              <Textarea
                id="ch-config"
                value={form.config}
                onChange={(e) => setForm((f) => ({ ...f, config: e.target.value }))}
                placeholder='{ "url": "https://hooks.example.com/…" }'
                spellCheck={false}
                className="min-h-24 resize-y font-mono text-sm"
              />
            </div>
            {configError ? (
              <p role="alert" className="text-sm text-destructive">
                config: {configError}
              </p>
            ) : null}
            <DialogFooter>
              <Button type="submit" disabled={create.isPending || !form.name.trim()}>
                {create.isPending ? "Creating…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
