// `index.tsx` — default export `Provision`. The admin panel chrome with a
// tab bar; routes to the phone PWA when the extension route starts with `pwa`.
import * as React from "react";
import "../app.css";
import {
  BlockShell,
  useExtensionRoute,
  useHostTheme,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";
import { LayoutGrid, MapPin, FileCode, Wand2, Eye } from "lucide-react";
import { EXTENSION_ID } from "../types";
import PwaApp from "../pwa";
import { DevicesTab, SitesTab, TemplatesTab, WizardTab, PagePreviewTab } from "./tabs";

type IconCmp = React.ComponentType<{ className?: string }>;
const TABS: ReadonlyArray<{ id: string; label: string; icon: IconCmp; render: () => React.ReactElement }> = [
  { id: "devices", label: "Devices", icon: LayoutGrid, render: () => <DevicesTab /> },
  { id: "sites", label: "Sites & Locations", icon: MapPin, render: () => <SitesTab /> },
  { id: "templates", label: "Templates", icon: FileCode, render: () => <TemplatesTab /> },
  { id: "wizard", label: "Provision wizard", icon: Wand2, render: () => <WizardTab /> },
  { id: "preview", label: "Page preview", icon: Eye, render: () => <PagePreviewTab /> },
];

export default function Provision(): React.ReactElement {
  return (
    <BlockShell>
      <ProvisionRouter />
    </BlockShell>
  );
}

// The router body without its own `BlockShell` wrapper, for mounting
// inside the host `Main` slot (which is already inside a BlockShell).
// `Main` dispatches the `provision`/`pwa` routes to this.
export function ProvisionRouter(): React.ReactElement {
  const route = useExtensionRoute();
  const slot = useSlotContext();
  const theme = useHostTheme();

  if (route === "pwa" || route?.startsWith("pwa/")) {
    return (
      <div data-ext-id={EXTENSION_ID} data-ext-slot={slot.slotId} data-ext-theme={theme.mode}>
        <PwaApp />
      </div>
    );
  }

  const initial = TABS.find((t) => route === `provision/${t.id}`)?.id ?? "devices";
  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      data-ext-theme={theme.mode}
      className="flex flex-col gap-4 p-4"
    >
      <AdminPanel initial={initial} />
    </div>
  );
}

function AdminPanel({ initial }: { initial: string }): React.ReactElement {
  const [active, setActive] = React.useState(initial);
  const tab = TABS.find((t) => t.id === active) ?? TABS[0]!;
  return (
    <>
      <div>
        <h3 className="text-lg font-semibold tracking-tight">Provisioning</h3>
        <p className="text-sm text-muted-foreground">Scan-to-dashboard device commissioning</p>
      </div>
      <nav className="flex flex-wrap gap-1 border-b border-border/60">
        {TABS.map((t) => {
          const Icon = t.icon;
          const on = t.id === active;
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => setActive(t.id)}
              className={
                "flex items-center gap-1.5 rounded-t-md px-3 py-2 text-sm transition-colors " +
                (on
                  ? "border-b-2 border-primary font-medium text-foreground"
                  : "text-muted-foreground hover:text-foreground")
              }
            >
              <Icon className="size-4" />
              {t.label}
            </button>
          );
        })}
      </nav>
      <div>{tab.render()}</div>
    </>
  );
}
