// `device-detail.tsx` — points list for a device with read-only trend/alarm
// state (set at provision time; tools don't expose per-point updates).
import * as React from "react";
import { pointsByDevice } from "../bc-api";
import type { PointRow } from "../bc-types";
import { Switch } from "../ui/switch";

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
    <table className="w-full text-sm">
      <thead>
        <tr className="text-left text-xs text-muted-foreground">
          <th className="px-2 py-1 font-medium">Point</th>
          <th className="px-2 py-1 font-medium">Widget</th>
          <th className="px-2 py-1 font-medium" title="set at provision time">Trend</th>
          <th className="px-2 py-1 font-medium" title="set at provision time">Alarm</th>
        </tr>
      </thead>
      <tbody>
        {points.map((p) => (
          <tr key={p.point_id} className="border-t border-border/40">
            <td className="px-2 py-1.5 text-foreground">
              {p.name}
              {p.unit ? <span className="text-muted-foreground"> ({p.unit})</span> : null}
            </td>
            <td className="px-2 py-1.5 text-muted-foreground">{p.widget}</td>
            <td className="px-2 py-1.5" title="set at provision time">
              <Switch checked={p.trend_on} label={`${p.name} trend`} disabled />
            </td>
            <td className="px-2 py-1.5" title="set at provision time">
              <Switch checked={p.alarm_on} label={`${p.name} alarm`} disabled />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
