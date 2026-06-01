// `identify.tsx` — decoded device summary card with a point preview list.
import * as React from "react";
import { Cpu } from "lucide-react";
import type { ScannedIdentity } from "../provision/bc-types";

export function Identify({ identity }: { identity: ScannedIdentity }): React.ReactElement {
  const t = identity.template;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-3 rounded-xl border border-border/60 bg-card p-4">
        <div className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Cpu className="size-6" />
        </div>
        <div className="min-w-0">
          <div className="truncate text-base font-semibold text-foreground">{t.display_name}</div>
          <div className="truncate text-sm text-muted-foreground">
            {identity.model} · {identity.network} · {t.category}
          </div>
          <div className="truncate font-mono text-xs text-muted-foreground">{identity.id}</div>
        </div>
      </div>

      <div className="rounded-xl border border-border/60 bg-card">
        <header className="border-b border-border/60 px-4 py-2 text-xs font-medium text-muted-foreground">
          {t.points.length} points · addr {identity.address}
        </header>
        <ul className="divide-y divide-border/40">
          {t.points.slice(0, 8).map((p) => (
            <li key={p.key} className="flex items-center justify-between px-4 py-2 text-sm">
              <span className="text-foreground">{p.name}</span>
              <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">{p.widget}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
