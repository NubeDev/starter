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
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type { CreateAlertRuleRequest } from "@/api/types";
import { useRuleMutations } from "@/features/alerts/useAlerts";

// Comparison operators a rule evaluates its query's scalar against.
const OPS = [
  { value: ">", label: "greater than" },
  { value: ">=", label: "≥" },
  { value: "<", label: "less than" },
  { value: "<=", label: "≤" },
  { value: "==", label: "equals" },
  { value: "!=", label: "not equals" },
];

// Create an alert rule: a SQL query whose scalar result is compared
// `op threshold`, must hold `for` seconds, evaluated every `interval`.
// Channels are attached after creation (the channel picker is its own
// step) — kept minimal here so a rule can be authored in one form.
export function AlertRuleDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { create } = useRuleMutations();
  const [form, setForm] = useState({
    name: "",
    query: "",
    op: ">",
    threshold: "0",
    for_secs: "60",
    interval_secs: "60",
  });

  const set = (k: keyof typeof form) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const body: CreateAlertRuleRequest = {
      name: form.name.trim(),
      query: form.query.trim(),
      op: form.op,
      threshold: Number(form.threshold) || 0,
      for_secs: Number(form.for_secs) || 0,
      interval_secs: Number(form.interval_secs) || 60,
      enabled: true,
    };
    create.mutate(body, { onSuccess: () => onOpenChange(false) });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-lg">
        <DialogHeader>
          <DialogTitle>New alert rule</DialogTitle>
          <DialogDescription>
            Fire when a query's value crosses a threshold.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="rule-name">Name</Label>
            <Input
              id="rule-name"
              value={form.name}
              onChange={(e) => set("name")(e.target.value)}
              required
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="rule-query">Query (returns one number)</Label>
            <Textarea
              id="rule-query"
              value={form.query}
              onChange={(e) => set("query")(e.target.value)}
              placeholder="select avg(temp) from readings where ts > now() - interval '5m'"
              spellCheck={false}
              className="min-h-20 resize-y font-mono text-sm"
              required
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="rule-op">Condition</Label>
              <Select value={form.op} onValueChange={set("op")}>
                <SelectTrigger id="rule-op">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {OPS.map((o) => (
                    <SelectItem key={o.value} value={o.value}>
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="rule-threshold">Threshold</Label>
              <Input
                id="rule-threshold"
                type="number"
                value={form.threshold}
                onChange={(e) => set("threshold")(e.target.value)}
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="rule-for">For (seconds)</Label>
              <Input
                id="rule-for"
                type="number"
                value={form.for_secs}
                onChange={(e) => set("for_secs")(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="rule-interval">Check every (seconds)</Label>
              <Input
                id="rule-interval"
                type="number"
                value={form.interval_secs}
                onChange={(e) => set("interval_secs")(e.target.value)}
              />
            </div>
          </div>
          {create.isError ? (
            <p role="alert" className="text-sm text-destructive">
              Couldn't create the rule.
            </p>
          ) : null}
          <DialogFooter>
            <Button
              type="submit"
              disabled={create.isPending || !form.name.trim() || !form.query.trim()}
            >
              {create.isPending ? "Creating…" : "Create rule"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
