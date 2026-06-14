import { useState } from "react";
import { Check, CircleCheck } from "lucide-react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { Finding } from "@/api/types";
import { useDateTime } from "@/datetime";
import {
  useDetections,
  useFindings,
  useFindingMutations,
} from "@/features/detections/useDetections";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// The findings feed: one row per spark, filterable by status and detection,
// with per-row Acknowledge / Resolve. A finding is a *workflow* item, not just
// a log line — hence the lifecycle actions inline.
const STATUSES = [
  { value: "open", label: "Open" },
  { value: "acknowledged", label: "Acknowledged" },
  { value: "resolved", label: "Resolved" },
  { value: "", label: "All statuses" },
];

export function FindingsTab() {
  const [status, setStatus] = useState("open");
  const [detectionId, setDetectionId] = useState("");
  const detections = useDetections();
  const { data, isPending, isError, error } = useFindings({
    status: status || undefined,
    detectionId: detectionId || undefined,
  });

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Select value={status} onValueChange={setStatus}>
          <SelectTrigger className="w-44" aria-label="Filter by status">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {STATUSES.map((s) => (
              <SelectItem key={s.value || "all"} value={s.value || "all"}>
                {s.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={detectionId || "all"}
          onValueChange={(v) => setDetectionId(v === "all" ? "" : v)}
        >
          <SelectTrigger className="w-56" aria-label="Filter by detection">
            <SelectValue placeholder="All detections" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All detections</SelectItem>
            {(detections.data ?? []).map((d) => (
              <SelectItem key={d.id} value={d.id}>
                {d.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {isPending ? (
        <Loading label="Loading findings…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : undefined}
        />
      ) : data.length === 0 ? (
        <Empty
          title="No findings"
          description="Findings appear as detections flag offending targets."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((f) => (
            <FindingRow key={f.id} finding={f} />
          ))}
        </ul>
      )}
    </div>
  );
}

// Status → colour token, matching the alert-events tinting convention.
function statusColor(status: string): string {
  switch (status) {
    case "open":
      return "var(--destructive)";
    case "acknowledged":
      return "var(--chart-4)";
    default:
      return "var(--chart-1)"; // resolved
  }
}

function FindingRow({ finding }: { finding: Finding }) {
  const { dateTime } = useDateTime();
  const { ack, resolve } = useFindingMutations();
  const color = statusColor(finding.status);
  const target = finding.target as Record<string, unknown> | null;
  const targetLabel =
    target && Object.keys(target).length > 0
      ? Object.entries(target)
          .map(([k, v]) => `${k}=${String(v)}`)
          .join(" · ")
      : "—";
  const why = renderContext(finding.context);
  const resolved = finding.status === "resolved";

  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span
        className="size-2 shrink-0 rounded-full"
        style={{ backgroundColor: color, boxShadow: `0 0 8px ${color}` }}
        aria-hidden
      />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">
          {targetLabel}
        </p>
        <p className="tabular truncate text-xs text-muted-foreground">
          value {finding.value ?? "—"}
          {why ? ` · ${why}` : ""}
        </p>
      </div>
      <Badge
        variant="outline"
        className="capitalize"
        style={{ color, borderColor: color }}
      >
        {finding.status}
      </Badge>
      <span className="tabular hidden text-xs text-muted-foreground sm:inline">
        {dateTime(finding.at)}
      </span>
      {!resolved && (
        <div className="flex items-center gap-1">
          {finding.status === "open" && (
            <Button
              variant="ghost"
              size="icon"
              aria-label="Acknowledge"
              title="Acknowledge"
              disabled={ack.isPending}
              onClick={() => ack.mutate({ id: finding.id })}
            >
              <Check className="size-4" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            aria-label="Resolve"
            title="Resolve"
            disabled={resolve.isPending}
            onClick={() => resolve.mutate({ id: finding.id })}
            className="text-muted-foreground hover:text-[var(--chart-1)]"
          >
            <CircleCheck className="size-4" />
          </Button>
        </div>
      )}
    </li>
  );
}

// The flagged row's derived columns, the "why" — a compact key=value summary
// (e.g. the zscore that tripped an anomaly). Numbers are rounded for display.
function renderContext(context: unknown): string {
  if (!context || typeof context !== "object") return "";
  return Object.entries(context as Record<string, unknown>)
    .filter(([k]) => k !== "value")
    .map(([k, v]) => {
      const val = typeof v === "number" ? roundish(v) : String(v);
      return `${k}=${val}`;
    })
    .join(" · ");
}

function roundish(n: number): string {
  return Number.isInteger(n) ? String(n) : n.toFixed(2);
}
