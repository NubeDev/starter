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
import { Switch } from "@nube/starter-ui-kit/components/switch";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import { useCreateFlow } from "@/features/flows/useFlows";
import { toCreateFlow, type FlowDraft } from "@/features/flows/flowDraft";

// Per-section placeholders — editable JSON skeletons the user replaces,
// shown only when the field is empty (like the SQL placeholder in
// Explore). They hint the ArkFlow config shape; they are not submitted
// unless the user keeps them, so this is not fabricated data (F0).
const PLACEHOLDER = {
  input: '{\n  "type": "generate",\n  "interval": "1s"\n}',
  pipeline: '[\n  { "type": "sql", "query": "select * from messages" }\n]',
  output: '{\n  "type": "stdout"\n}',
};

type Section = "input" | "pipeline" | "output";

// Authoring dialog for a flow's ArkFlow config. Name + enabled, plus three
// JSON section editors (input / pipeline / output) the backend stores
// opaquely. Each section is validated client-side (`toCreateFlow`); the
// failing section's tab is flagged so the user lands on the right editor.
export function FlowFormDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const create = useCreateFlow();
  const [draft, setDraft] = useState<FlowDraft>({
    name: "",
    enabled: true,
    input: "",
    pipeline: "",
    output: "",
  });
  const [badSection, setBadSection] = useState<Section | null>(null);
  const [error, setError] = useState<string | null>(null);

  const set = <K extends keyof FlowDraft>(k: K, v: FlowDraft[K]) =>
    setDraft((d) => ({ ...d, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    setBadSection(null);
    const built = toCreateFlow(draft);
    if (!built.ok) {
      setBadSection(built.field);
      setError(`${built.field}: ${built.error}`);
      return;
    }
    create.mutate(built.value, {
      onSuccess: () => {
        onOpenChange(false);
        setDraft({ name: "", enabled: true, input: "", pipeline: "", output: "" });
      },
      onError: () => setError("Couldn't create the flow."),
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-2xl">
        <DialogHeader>
          <DialogTitle>New flow</DialogTitle>
          <DialogDescription>
            A flow is a long-running ingestion pipeline. Author its ArkFlow
            input, pipeline, and output.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={onSubmit}>
          <div className="flex items-end gap-4">
            <div className="flex-1 space-y-1.5">
              <Label htmlFor="flow-name">Name</Label>
              <Input
                id="flow-name"
                value={draft.name}
                onChange={(e) => set("name", e.target.value)}
                placeholder="weather → timescale"
                required
              />
            </div>
            <label className="flex items-center gap-2 pb-2 text-sm">
              <Switch
                checked={draft.enabled}
                onCheckedChange={(v) => set("enabled", v)}
              />
              Enabled
            </label>
          </div>

          <Tabs defaultValue="input">
            <TabsList>
              {(["input", "pipeline", "output"] as Section[]).map((s) => (
                <TabsTrigger
                  key={s}
                  value={s}
                  className={`capitalize ${badSection === s ? "text-destructive" : ""}`}
                >
                  {s}
                </TabsTrigger>
              ))}
            </TabsList>
            {(["input", "pipeline", "output"] as Section[]).map((s) => (
              <TabsContent key={s} value={s}>
                <Textarea
                  value={draft[s]}
                  onChange={(e) => set(s, e.target.value)}
                  placeholder={PLACEHOLDER[s]}
                  spellCheck={false}
                  className="min-h-40 resize-y font-mono text-sm"
                  aria-label={`Flow ${s} config`}
                />
              </TabsContent>
            ))}
          </Tabs>

          {error ? (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={create.isPending || !draft.name.trim()}>
              {create.isPending ? "Creating…" : "Create flow"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
