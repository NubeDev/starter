import { Badge } from "@nube/starter-ui-kit/components/badge";

import type { Change } from "@/api/types";
import { useDateTime } from "@/datetime";
import { actorLabel, opLabel } from "@/features/audit/actorLabel";
import { ChangeDiff } from "@/features/audit/ChangeDiff";

// A newest-first list of changes, each showing who/what/when and an expandable
// before -> after diff. Shared by the admin audit screen and a per-resource
// History tab. Empty/loading/error are the caller's concern (F0).
export function ChangeList({ changes }: { changes: Change[] }) {
  const { dateTime } = useDateTime();

  if (changes.length === 0) {
    return (
      <p className="py-8 text-center text-sm text-muted-foreground">
        No changes recorded yet.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-3">
      {changes.map((change) => (
        <li
          key={change.id}
          className="rounded-md border border-border bg-card p-3"
        >
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <Badge variant="secondary">{opLabel(change.op)}</Badge>
            <span className="font-medium">{change.resource.kind}</span>
            {change.resource.id ? (
              <span className="font-mono text-xs text-muted-foreground">
                {change.resource.id}
              </span>
            ) : null}
            <span className="ml-auto text-xs text-muted-foreground">
              {actorLabel(change.actor)} · {dateTime(change.at)}
            </span>
          </div>
          <div className="mt-2">
            <ChangeDiff
              before={change.before as Record<string, unknown> | null}
              after={change.after as Record<string, unknown> | null}
            />
          </div>
        </li>
      ))}
    </ul>
  );
}
