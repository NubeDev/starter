// `ui/sidebar.tsx` — compact Sidebar panel for `com.rubix.example`.

import * as React from "react";

import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID } from "./types";
import type { ExtensionDetail } from "./types";
import { fetchExtensionDetail } from "./lib/detail";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { Badge } from "./components/ui/badge";

export default function Sidebar(): React.ReactElement {
  return (
    <BlockShell>
      <SidebarInner />
    </BlockShell>
  );
}

function SidebarInner(): React.ReactElement {
  const slot = useSlotContext();
  const [detail, setDetail] = React.useState<ExtensionDetail | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    fetchExtensionDetail()
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const c = detail?.manifest?.contributes ?? {};
  const toolCount = (c.tools ?? []).length;
  const tableCount = (c.warehouse_tables ?? []).length;
  const ruleCount = (c.anomaly_rules ?? []).length;
  const version = detail?.manifest?.version ?? null;

  return (
    <Card
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      className="mx-2 my-1 py-3"
    >
      <CardHeader className="px-3 py-0">
        <div className="flex items-baseline justify-between gap-2">
          <CardTitle className="text-xs">Rubix Example</CardTitle>
          {version ? (
            <span className="text-muted-foreground text-[0.65rem]">v{version}</span>
          ) : null}
        </div>
      </CardHeader>

      <CardContent className="px-3 py-0">
        {error ? (
          <p role="alert" className="text-sm text-destructive">{error}</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            <Badge variant="outline" className="text-[0.65rem] px-1.5 py-0">
              tools: {toolCount}
            </Badge>
            <Badge variant="outline" className="text-[0.65rem] px-1.5 py-0">
              tables: {tableCount}
            </Badge>
            <Badge variant="outline" className="text-[0.65rem] px-1.5 py-0">
              rules: {ruleCount}
            </Badge>
          </div>
        )}

        <a
          href="/extensions"
          className="text-xs text-primary hover:underline mt-2 inline-block"
        >
          open full panel →
        </a>
      </CardContent>
    </Card>
  );
}
