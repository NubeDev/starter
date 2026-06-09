import { useAlertEvents } from "@/features/alerts/useAlerts";
import { useDateTime } from "@/datetime";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Fired-alert history (read-only). Each event is a rule transition with
// its value and whether it was silenced/notified. A "firing" transition is
// tinted destructive, "resolved" emerald.
export function EventsTab() {
  const { data, isPending, isError, error } = useAlertEvents();
  const { dateTime } = useDateTime();

  if (isPending) return <Loading label="Loading events…" />;
  if (isError) {
    return <ErrorState message={error instanceof Error ? error.message : undefined} />;
  }
  if (data.length === 0) {
    return <Empty title="No alert events" description="Fired alerts will appear here." />;
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {data.map((ev) => {
        const firing = ev.transition.toLowerCase().includes("fir");
        const color = firing ? "var(--destructive)" : "var(--chart-1)";
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
