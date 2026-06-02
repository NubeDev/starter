// `device-detail.tsx` — points list for a device with read-only trend/alarm
// state (set at provision time; tools don't expose per-point updates).
//
// Trend/alarm are shown as read-only status pills (NOT switches): there is
// no per-point update tool, so an interactive toggle would be a dead
// control. The pills + caption make the read-only intent obvious.
import * as React from "react";
import { Check, Minus } from "lucide-react";
import { pointsByDevice } from "../bc-api";
import type { PointRow } from "../bc-types";

// A read-only on/off indicator. Green check when on, muted dash when off.
function StatePill({ on, label }: { on: boolean; label: string }): React.ReactElement {
  return (
    <span
      aria-label={`${label}: ${on ? "on" : "off"}`}
      className={
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium " +
        (on ? "bg-emerald-500/10 text-emerald-400" : "bg-muted/40 text-muted-foreground")
      }
    >
      {on ? <Check className="size-3" /> : <Minus className="size-3" />}
      {on ? "On" : "Off"}
    </span>
  );
}

export function DeviceDetail({ deviceId }: { deviceId: string }): React.ReactElement {
  const [points, setPoints] = React.useState<ReadonlyArray<PointRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    pointsByDevice(deviceId)
      .then((rs) => !cancelled && setPoints(rs))
      .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [deviceId]);

  if (error) return <p className="px-3 py-2 text-sm text-destructive">{error}</p>;
  if (loading) return <p className="px-3 py-2 text-sm italic text-muted-foreground">loading…</p>;
  if (points.length === 0) return <p className="px-3 py-2 text-sm italic text-muted-foreground">No points.</p>;

  return (
    <div className="flex flex-col gap-2">
      <table className="w-full text-sm">
        <thead>
          <tr className="text-left">
            <th className="px-2 py-1"><span className="ext-eyebrow">Point</span></th>
            <th className="px-2 py-1"><span className="ext-eyebrow">Widget</span></th>
            <th className="px-2 py-1"><span className="ext-eyebrow">Trend</span></th>
            <th className="px-2 py-1"><span className="ext-eyebrow">Alarm</span></th>
          </tr>
        </thead>
        <tbody>
          {points.map((p) => (
            <tr key={p.point_id} className="border-t border-border/40">
              <td className="px-2 py-1.5 text-foreground">
                {p.name}
                {p.unit ? <span className="text-muted-foreground"> ({p.unit})</span> : null}
              </td>
              <td className="px-2 py-1.5 font-mono text-xs text-muted-foreground">{p.widget}</td>
              <td className="px-2 py-1.5">
                <StatePill on={p.trend_on} label={`${p.name} trend`} />
              </td>
              <td className="px-2 py-1.5">
                <StatePill on={p.alarm_on} label={`${p.name} alarm`} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="px-2 text-xs italic text-muted-foreground">
        Trend &amp; alarm are set when the device is provisioned and are read-only here.
      </p>
    </div>
  );
}
