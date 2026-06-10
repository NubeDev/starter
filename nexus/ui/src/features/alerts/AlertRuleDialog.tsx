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

// Comparison operators a rule evaluates its query's scalar against. The `value`
// is the operator name the backend's comparator understands (gt/gte/lt/lte/eq/ne);
// the label is the human-readable symbol shown in the picker.
const OPS = [
  { value: "gt", label: "greater than (>)" },
  { value: "gte", label: "at least (≥)" },
  { value: "lt", label: "less than (<)" },
  { value: "lte", label: "at most (≤)" },
  { value: "eq", label: "equals (=)" },
  { value: "ne", label: "not equals (≠)" },
];

// What state to take when the query returns no rows / fails to run. Mirrors the
// backend per-rule no_data_policy / exec_error_policy: ok = treat as healthy,
// alerting = treat as breaching, keep_last = hold the prior state.
const POLICIES = [
  { value: "ok", label: "Treat as OK" },
  { value: "alerting", label: "Treat as alerting" },
  { value: "keep_last", label: "Keep last state" },
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
    op: "gt",
    threshold: "0",
    for_secs: "60",
    interval_secs: "60",
    no_data_policy: "ok",
    exec_error_policy: "ok",
    message_template: "",
  });

  const set = (k: keyof typeof form) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const template = form.message_template.trim();
    const body: CreateAlertRuleRequest = {
      name: form.name.trim(),
      query: form.query.trim(),
      op: form.op,
      threshold: Number(form.threshold) || 0,
      for_secs: Number(form.for_secs) || 0,
      interval_secs: Number(form.interval_secs) || 60,
      enabled: true,
      no_data_policy: form.no_data_policy,
      exec_error_policy: form.exec_error_policy,
      message_template: template === "" ? null : template,
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
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="rule-no-data">When no data</Label>
              <Select
                value={form.no_data_policy}
                onValueChange={set("no_data_policy")}
              >
                <SelectTrigger id="rule-no-data">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICIES.map((p) => (
                    <SelectItem key={p.value} value={p.value}>
                      {p.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="rule-exec-error">On query error</Label>
              <Select
                value={form.exec_error_policy}
                onValueChange={set("exec_error_policy")}
              >
                <SelectTrigger id="rule-exec-error">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICIES.map((p) => (
                    <SelectItem key={p.value} value={p.value}>
                      {p.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="rule-template">Message template (optional)</Label>
            <Textarea
              id="rule-template"
              value={form.message_template}
              onChange={(e) => set("message_template")(e.target.value)}
              placeholder="Alert {{rule_name}} is {{state}} (value {{value}} {{op}} threshold {{threshold}})"
              spellCheck={false}
              className="min-h-16 resize-y font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Tokens: {"{{rule_name}} {{state}} {{value}} {{op}} {{threshold}}"}
            </p>
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
