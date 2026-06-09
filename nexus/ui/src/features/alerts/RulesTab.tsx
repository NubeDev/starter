import { useState } from "react";
import { Bell, Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { AlertRuleDetail } from "@/api/types";
import { useAlertRules, useRuleMutations } from "@/features/alerts/useAlerts";
import { AlertRuleDialog } from "@/features/alerts/AlertRuleDialog";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Threshold rules: list with their condition, enabled state, and delete.
export function RulesTab() {
  const { data, isPending, isError, error } = useAlertRules();
  const { remove } = useRuleMutations();
  const [creating, setCreating] = useState(false);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex justify-end">
        <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
          <Plus className="size-4" />
          New rule
        </Button>
      </div>

      {isPending ? (
        <Loading label="Loading rules…" />
      ) : isError ? (
        <ErrorState message={error instanceof Error ? error.message : undefined} />
      ) : data.length === 0 ? (
        <Empty title="No alert rules" description="Create a threshold rule to begin." />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((rule) => (
            <RuleRow
              key={rule.id}
              rule={rule}
              onRemove={() => remove.mutate(rule.id)}
              removing={remove.isPending}
            />
          ))}
        </ul>
      )}

      <AlertRuleDialog open={creating} onOpenChange={setCreating} />
    </div>
  );
}

function RuleRow({
  rule,
  onRemove,
  removing,
}: {
  rule: AlertRuleDetail;
  onRemove: () => void;
  removing: boolean;
}) {
  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span
        className="grid size-9 place-items-center rounded-lg"
        style={{
          background: rule.enabled
            ? "color-mix(in oklab, var(--primary) 15%, transparent)"
            : "var(--muted)",
          color: rule.enabled ? "var(--primary)" : "var(--muted-foreground)",
        }}
      >
        <Bell className="size-4" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">{rule.name}</p>
        <p className="tabular truncate text-xs text-muted-foreground">
          value {rule.op} {rule.threshold} for {rule.for_secs}s · every{" "}
          {rule.interval_secs}s · {rule.channel_ids.length} channel
          {rule.channel_ids.length === 1 ? "" : "s"}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${rule.name}`}
        disabled={removing}
        onClick={onRemove}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}
