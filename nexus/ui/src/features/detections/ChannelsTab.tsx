import { useMemo, useState, type FormEvent } from "react";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import {
  buildChannelConfig,
  CHANNEL_KINDS,
  channelKind,
} from "@/features/detections/channelKinds";
import {
  useChannelMutations,
  useChannels,
} from "@/features/detections/useNotify";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Notification channels: list with kind + delete, and a create form driven by a
// kind picker plus a schema-driven set of inputs per kind (webhook/slack/email),
// so the operator fills typed fields instead of hand-writing a config JSON blob.
// An alert-type detection references these channels by id. Secret fields (Slack
// URL, SMTP password) are never echoed back by the API.
export function ChannelsTab() {
  const { data, isPending, isError, error } = useChannels();
  const { create, remove } = useChannelMutations();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState("webhook");
  const [values, setValues] = useState<Record<string, string | boolean>>({});

  const spec = useMemo(() => channelKind(kind), [kind]);

  function reset() {
    setName("");
    setKind("webhook");
    setValues({});
  }

  function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    create.mutate(
      { name: name.trim(), kind, config: buildChannelConfig(kind, values) },
      {
        onSuccess: () => {
          setOpen(false);
          reset();
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
        <Empty
          title="No channels"
          description="Add a channel to notify on detections."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((c) => (
            <li key={c.id} className="glass flex items-center gap-3 rounded-lg px-4 py-3">
              <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
                <Send className="size-4" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-foreground">{c.name}</p>
                <p className="text-xs text-muted-foreground">
                  {channelKind(c.kind)?.label ?? c.kind}
                </p>
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
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="ch-kind">Kind</Label>
              <Select
                value={kind}
                onValueChange={(k) => {
                  setKind(k);
                  setValues({});
                }}
              >
                <SelectTrigger id="ch-kind">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CHANNEL_KINDS.map((k) => (
                    <SelectItem key={k.kind} value={k.kind}>
                      {k.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {spec ? (
                <p className="text-xs text-muted-foreground">{spec.description}</p>
              ) : null}
            </div>

            {spec?.fields.map((field) =>
              field.type === "checkbox" ? (
                <label
                  key={field.key}
                  className="flex items-center gap-2 text-sm text-foreground"
                >
                  <input
                    type="checkbox"
                    checked={values[field.key] === true}
                    onChange={(e) =>
                      setValues((v) => ({ ...v, [field.key]: e.target.checked }))
                    }
                  />
                  {field.label}
                </label>
              ) : (
                <div key={field.key} className="space-y-1.5">
                  <Label htmlFor={`ch-${field.key}`}>{field.label}</Label>
                  <Input
                    id={`ch-${field.key}`}
                    type={field.type === "password" ? "password" : field.type}
                    value={(values[field.key] as string) ?? ""}
                    onChange={(e) =>
                      setValues((v) => ({ ...v, [field.key]: e.target.value }))
                    }
                    placeholder={field.placeholder}
                    required={field.required}
                    spellCheck={false}
                  />
                </div>
              ),
            )}

            <DialogFooter>
              <Button type="submit" disabled={create.isPending || !name.trim()}>
                {create.isPending ? "Creating…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
