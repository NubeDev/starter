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
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type {
  AgentDetail,
  CreateAgentRequest,
  UpdateAgentRequest,
} from "@/api/types";
import { useCreateAgent, useUpdateAgent } from "@/features/agents/useAgents";

// The agent's `backend` selects which tier the run dispatches to. CLI agents
// drive the locally-installed coding-agent binary via its OWN login — no
// provider API key needed in the platform (the headline mode). Raw providers
// call the HTTP API and need a key configured server-side. `model` is then a
// tier alias (large/medium/small) or a concrete id for that backend.
const CLI_BACKENDS = [
  { value: "claude", label: "Claude Code (CLI)" },
  { value: "codex", label: "Codex (CLI)" },
  { value: "gemini", label: "Gemini (CLI)" },
  { value: "ollama", label: "Ollama (local)" },
];
const PROVIDER_BACKENDS = [
  { value: "anthropic", label: "Anthropic API" },
  { value: "openai", label: "OpenAI API" },
];

// Create / edit an agent: name + backend + model + an optional system
// prompt. When `agent` is supplied the dialog is in edit mode and submits a
// partial update; otherwise it creates. Closes on success.
export function AgentFormDialog({
  open,
  onOpenChange,
  agent,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  agent?: AgentDetail;
}) {
  const create = useCreateAgent();
  const update = useUpdateAgent();
  const editing = agent != null;

  const [form, setForm] = useState({
    name: agent?.name ?? "",
    backend: agent?.backend ?? "claude",
    model: agent?.model ?? "",
    system_prompt: agent?.system_prompt ?? "",
  });

  const set = (k: keyof typeof form) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const model = form.model.trim();
    const prompt = form.system_prompt.trim();

    if (editing) {
      const patch: UpdateAgentRequest = {
        name: form.name.trim(),
        backend: form.backend,
        model: model === "" ? null : model,
        system_prompt: prompt === "" ? null : prompt,
      };
      update.mutate(
        { id: agent.id, patch },
        { onSuccess: () => onOpenChange(false) },
      );
      return;
    }

    const body: CreateAgentRequest = {
      name: form.name.trim(),
      backend: form.backend,
      model: model === "" ? undefined : model,
      system_prompt: prompt === "" ? undefined : prompt,
    };
    create.mutate(body, { onSuccess: () => onOpenChange(false) });
  }

  const busy = create.isPending || update.isPending;
  const failed = create.isError || update.isError;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>{editing ? "Edit agent" : "New agent"}</DialogTitle>
          <DialogDescription>
            Configure an AI endpoint to chat with.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <Field
            id="agent-name"
            label="Name"
            value={form.name}
            onChange={set("name")}
            required
          />
          <div className="space-y-1.5">
            <Label htmlFor="agent-backend">Backend</Label>
            <Select value={form.backend} onValueChange={set("backend")}>
              <SelectTrigger id="agent-backend">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Local CLI — no API key</SelectLabel>
                  {CLI_BACKENDS.map((b) => (
                    <SelectItem key={b.value} value={b.value}>
                      {b.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
                <SelectGroup>
                  <SelectLabel>Provider API — needs a key</SelectLabel>
                  {PROVIDER_BACKENDS.map((b) => (
                    <SelectItem key={b.value} value={b.value}>
                      {b.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              CLI agents use the locally-installed tool's own login — no provider
              key needed.
            </p>
          </div>
          <Field
            id="agent-model"
            label="Model"
            value={form.model}
            onChange={set("model")}
            placeholder="large | medium | small | or a concrete id like claude-opus-4-8"
          />
          <div className="space-y-1.5">
            <Label htmlFor="agent-prompt">System prompt (optional)</Label>
            <Textarea
              id="agent-prompt"
              value={form.system_prompt}
              onChange={(e) => set("system_prompt")(e.target.value)}
              placeholder="You are a helpful assistant for…"
              className="min-h-20 resize-y text-sm"
            />
          </div>
          {failed ? (
            <p role="alert" className="text-sm text-destructive">
              {editing
                ? "Couldn't save the agent."
                : "Couldn't create the agent."}
            </p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={busy || !form.name.trim()}>
              {busy
                ? editing
                  ? "Saving…"
                  : "Creating…"
                : editing
                  ? "Save"
                  : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Field({
  id,
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  required,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
  required?: boolean;
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        autoComplete="off"
        required={required}
      />
    </div>
  );
}
