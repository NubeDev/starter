import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import {
  DASHBOARD_ACCENTS,
  DASHBOARD_ICONS,
  dashboardIcon,
} from "@/features/dashboards/appearance";

// The dashboard appearance fields — name, icon, accent — as one controlled,
// presentational component. It owns no data or mutations; the create dialog
// and the edit dialog both render it and wire their own submit. This is the
// single place the icon grid + accent swatches live, so the two flows can't
// drift.
export interface DashboardFormValues {
  name: string;
  icon: string;
  accent: string;
}

export function DashboardForm({
  values,
  onChange,
  nameId = "dashboard-name",
  autoFocusName = true,
}: {
  values: DashboardFormValues;
  onChange: (next: DashboardFormValues) => void;
  nameId?: string;
  autoFocusName?: boolean;
}) {
  const set = (patch: Partial<DashboardFormValues>) =>
    onChange({ ...values, ...patch });

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor={nameId}>Name</Label>
        <Input
          id={nameId}
          value={values.name}
          onChange={(e) => set({ name: e.target.value })}
          placeholder="Cold chain"
          autoFocus={autoFocusName}
          required
        />
      </div>

      <div className="space-y-2">
        <Label>Icon</Label>
        <div className="flex flex-wrap gap-2">
          {DASHBOARD_ICONS.map((name) => {
            const Icon = dashboardIcon(name);
            const active = values.icon === name;
            return (
              <button
                key={name}
                type="button"
                aria-label={name}
                aria-pressed={active}
                onClick={() => set({ icon: name })}
                className={`flex size-9 items-center justify-center rounded-lg border transition-colors ${
                  active
                    ? "border-primary/50 bg-primary/10 text-primary"
                    : "border-border/60 bg-card/40 text-muted-foreground hover:border-border hover:text-foreground"
                }`}
              >
                <Icon className="size-4" />
              </button>
            );
          })}
        </div>
      </div>

      <div className="space-y-2">
        <Label>Accent</Label>
        <div className="flex items-center gap-2.5">
          {DASHBOARD_ACCENTS.map((accent) => {
            const active = values.accent === accent;
            return (
              <button
                key={accent}
                type="button"
                aria-label={`accent ${accent}`}
                aria-pressed={active}
                onClick={() => set({ accent })}
                className={`size-7 rounded-full transition-transform hover:scale-110 ${
                  active
                    ? "ring-2 ring-foreground/80 ring-offset-2 ring-offset-background"
                    : ""
                }`}
                style={{ background: `hsl(${accent})` }}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}
