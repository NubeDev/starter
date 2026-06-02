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
import { DevicesTab, SitesTab, TemplatesTab, WizardTab, PagePreviewTab, DevicePage } from "./tabs";

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

// Read the device id a deep-link / share URL points at, if the current
// URL is the device page (`…/provision/device?id=…`). Returns null on the
// list tabs. Lets a scanned/shared link land directly on the device page.
function readDeviceFromUrl(): string | null {
  if (typeof window === "undefined") return null;
  const { pathname, search } = window.location;
  if (!pathname.includes("/provision/device")) return null;
  return new URLSearchParams(search).get("id");
}

function AdminPanel({ initial }: { initial: string }): React.ReactElement {
  const [active, setActive] = React.useState(initial);
  // The device page is an overlay route driven by the URL (`?id=…`), so a
  // shared/scanned link or browser back/forward selects it. `gotoDevice` /
  // `gotoDevicesList` in `nav.ts` push history + dispatch popstate.
  const [deviceId, setDeviceId] = React.useState<string | null>(() => readDeviceFromUrl());
  React.useEffect(() => {
    const onNav = () => setDeviceId(readDeviceFromUrl());
    window.addEventListener("popstate", onNav);
    return () => window.removeEventListener("popstate", onNav);
  }, []);

  const tab = TABS.find((t) => t.id === active) ?? TABS[0]!;

  if (deviceId) {
    return (
      <div className="ext-dash-shell flex flex-col gap-4">
        <DevicePage deviceId={deviceId} />
      </div>
    );
  }

  return (
    <div className="ext-dash-shell flex flex-col gap-4">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div className="flex flex-col gap-0.5">
          <span className="ext-eyebrow">IoT Provisioning</span>
          <h3 className="text-xl font-semibold tracking-tight text-foreground">Provisioning</h3>
          <p className="text-sm text-muted-foreground">Scan-to-dashboard device commissioning</p>
        </div>
        <div className="flex items-center gap-2">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-border/60 bg-muted/30 px-3 py-1 text-xs text-muted-foreground">
            <span className="inline-block size-1.5 rounded-full bg-emerald-500 shadow-[0_0_10px_2px_color-mix(in_oklab,var(--color-primary)_50%,transparent)]" />
            live
          </span>
          <span className="rounded-full border border-border/60 px-2.5 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            v2.4.0
          </span>
        </div>
      </header>

      <nav className="flex flex-wrap gap-1 rounded-xl border border-border/60 bg-muted/20 p-1">
        {TABS.map((t) => {
          const Icon = t.icon;
          const on = t.id === active;
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => setActive(t.id)}
              aria-current={on ? "page" : undefined}
              className={
                "flex cursor-pointer items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm transition-colors duration-200 " +
                (on
                  ? "bg-card font-medium text-foreground shadow-sm ring-1 ring-border/60"
                  : "text-muted-foreground hover:bg-card/50 hover:text-foreground")
              }
            >
              <Icon className="size-4" />
              {t.label}
            </button>
          );
        })}
      </nav>

      <div>{tab.render()}</div>
    </div>
  );
}
