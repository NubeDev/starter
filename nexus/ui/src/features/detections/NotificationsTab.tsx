import { useNotifyEvents } from "@/features/detections/useNotify";
import { useDateTime } from "@/datetime";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Notification history (read-only). Each event is a finding transition the
// runner tried to deliver, with its value and whether it was silenced/notified.
// An "opened" transition is tinted destructive, "resolved" emerald.
export function NotificationsTab() {
  const { data, isPending, isError, error } = useNotifyEvents();
  const { dateTime } = useDateTime();

  if (isPending) return <Loading label="Loading notifications…" />;
  if (isError) {
    return <ErrorState message={error instanceof Error ? error.message : undefined} />;
  }
  if (data.length === 0) {
    return (
      <Empty
        title="No notifications"
        description="Deliveries from alert-type detections will appear here."
      />
    );
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {data.map((ev) => {
        const opened = ev.transition.toLowerCase() === "opened";
        const color = opened ? "var(--destructive)" : "var(--chart-1)";
        return (
          <li
            key={ev.id}
            className="glass flex items-center gap-3 rounded-lg px-4 py-2.5"
          >
            <span
              className="size-2 shrink-0 rounded-full"
              style={{ backgroundColor: color, boxShadow: `0 0 8px ${color}` }}
              aria-hidden
            />
            <span className="capitalize" style={{ color }}>
              {ev.transition}
            </span>
            <span className="tabular text-sm text-muted-foreground">
              {ev.value ?? "—"}
            </span>
            <span className="ms-auto flex items-center gap-2 text-xs text-muted-foreground">
              {ev.silenced ? <span>silenced</span> : null}
              {ev.notified ? <span>notified</span> : null}
              <span className="tabular">{dateTime(ev.at)}</span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
