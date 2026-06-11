import { useState } from "react";
import { Pencil, Play, Plus, Radar, Trash2 } from "lucide-react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { DetectionDetail, DetectionStats } from "@/api/types";
import {
  useDetectionMutations,
  useDetections,
  useDetectionStats,
} from "@/features/detections/useDetections";
import { DetectionDialog } from "@/features/detections/DetectionDialog";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Detections: the scheduled analytic rules. List with their schedule + target
// mapping, run stats, run-now (off-schedule), edit, and delete (its findings
// cascade).
export function DetectionsTab() {
  const { data, isPending, isError, error } = useDetections();
  const { remove, run } = useDetectionMutations();
  const [creating, setCreating] = useState(false);
  // The detection being edited (null = the create dialog). One dialog instance
  // serves both — edit mode is just a prefilled create form that PUTs.
  const [editing, setEditing] = useState<DetectionDetail | null>(null);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex justify-end">
        <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
          <Plus className="size-4" />
          New detection
        </Button>
      </div>

      {isPending ? (
        <Loading label="Loading detections…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : undefined}
        />
      ) : data.length === 0 ? (
        <Empty
          title="No detections"
          description="Create one to run an insight on a schedule and emit findings."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((d) => (
            <DetectionRow
              key={d.id}
              detection={d}
              onEdit={() => setEditing(d)}
              onRun={() => run.mutate(d.id)}
              running={run.isPending}
              onRemove={() => remove.mutate(d.id)}
              removing={remove.isPending}
            />
          ))}
        </ul>
      )}

      {/* Create dialog. */}
      <DetectionDialog open={creating} onOpenChange={setCreating} />
      {/* Edit dialog — mounted only while a target is selected so it re-seeds
          from that detection; closing clears the target. */}
      {editing ? (
        <DetectionDialog
          open
          detection={editing}
          onOpenChange={(o) => {
            if (!o) setEditing(null);
          }}
        />
      ) : null}
    </div>
  );
}

function DetectionRow({
  detection,
  onEdit,
  onRun,
  running,
  onRemove,
  removing,
}: {
  detection: DetectionDetail;
  onEdit: () => void;
  onRun: () => void;
  running: boolean;
  onRemove: () => void;
  removing: boolean;
}) {
  const targets = detection.target_columns.join(", ") || "—";
  const flag = detection.flag_column.trim() || "every row";
  const federated = Array.isArray(detection.sources) && detection.sources.length > 0;
  const stats = useDetectionStats(detection.id);

  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span
        className="grid size-9 place-items-center rounded-lg"
        style={{
          background: detection.enabled
            ? "color-mix(in oklab, var(--primary) 15%, transparent)"
            : "var(--muted)",
          color: detection.enabled
            ? "var(--primary)"
            : "var(--muted-foreground)",
        }}
      >
        <Radar className="size-4" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="truncate text-sm font-medium text-foreground">
            {detection.name}
          </p>
          {stats.data && stats.data.open > 0 ? (
            <Badge
              variant="outline"
              style={{
                color: "var(--destructive)",
                borderColor: "var(--destructive)",
              }}
            >
              {stats.data.open} open
            </Badge>
          ) : null}
        </div>
        <p className="tabular truncate text-xs text-muted-foreground">
          flag: {flag} · target: {targets} · every {detection.interval_secs}s
          {federated ? " · federated" : ""}
        </p>
        <p className="tabular truncate text-xs text-muted-foreground">
          {renderStats(stats.data)}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Edit ${detection.name}`}
        title="Edit"
        onClick={onEdit}
        className="text-muted-foreground hover:text-foreground"
      >
        <Pencil className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Run ${detection.name} now`}
        title="Run now"
        disabled={running}
        onClick={onRun}
        className="text-muted-foreground hover:text-primary"
      >
        <Play className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${detection.name}`}
        disabled={removing}
        onClick={onRemove}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}

// The glanceable run line: findings totals + last spark + next run. Undefined
// while the stats query loads, so the row stays quiet rather than flashing.
function renderStats(s: DetectionStats | undefined): string {
  if (!s) return "loading stats…";
  const parts: string[] = [];
  parts.push(`${s.total} finding${s.total === 1 ? "" : "s"}`);
  if (s.acknowledged > 0) parts.push(`${s.acknowledged} ack'd`);
  if (s.resolved > 0) parts.push(`${s.resolved} resolved`);
  parts.push(
    s.last_finding_at ? `last spark ${relative(s.last_finding_at)}` : "no sparks yet",
  );
  parts.push(`next run ${relative(s.next_eval_at)}`);
  return parts.join(" · ");
}

// A compact relative time ("3m ago" / "in 4m"). Coarse buckets — this is a
// glance, not a clock. Uses Date.now at render; the query refetches on the
// findings cadence so it doesn't drift far.
function relative(iso: string): string {
  const diffMs = new Date(iso).getTime() - Date.now();
  const future = diffMs > 0;
  const abs = Math.abs(diffMs);
  const mins = Math.round(abs / 60000);
  let body: string;
  if (abs < 45000) body = "just now";
  else if (mins < 60) body = `${mins}m`;
  else if (mins < 1440) body = `${Math.round(mins / 60)}h`;
  else body = `${Math.round(mins / 1440)}d`;
  if (body === "just now") return body;
  return future ? `in ${body}` : `${body} ago`;
}
