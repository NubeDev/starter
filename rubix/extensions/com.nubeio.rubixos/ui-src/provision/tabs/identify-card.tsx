// `identify-card.tsx` — desktop decoded-device summary for the wizard.
import * as React from "react";
import { Cpu } from "lucide-react";
import type { ScannedIdentity } from "../bc-types";

export function IdentifyCard({ identity }: { identity: ScannedIdentity }): React.ReactElement {
  const t = identity.template;
  return (
    <div className="rounded-lg border border-border/60 bg-card p-4">
      <div className="flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Cpu className="size-5" />
        </div>
        <div>
          <div className="font-semibold text-foreground">{t.display_name}</div>
          <div className="text-sm text-muted-foreground">
            {identity.model} · {identity.network} · {t.category} · addr {identity.address}
          </div>
          <div className="font-mono text-xs text-muted-foreground">{identity.id}</div>
        </div>
      </div>
      <div className="mt-3 flex flex-wrap gap-1.5">
        {t.points.map((p) => (
          <span key={p.key} className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            {p.name} · {p.widget}
          </span>
        ))}
      </div>
    </div>
  );
}
