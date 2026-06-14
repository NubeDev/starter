// `main.tsx` — the single `main`-slot component for `com.acme.devices`, a tiny
// router over the extension's sub-pages.
//
// The host mounts ONE component per extension into the `main` slot and forwards
// the URL tail after `/x/:extId/` as `SlotContext.route` (see nexus-ui's
// `ExtensionPage` + the SDK's `useSlotContext().route`). It does NOT pick a
// different exposed component per URL — so multi-page extensions dispatch their
// own sub-pages here, exactly how `com.nexus.demo`'s `Main` does.
//
// Routes (relative to `/x/com.acme.devices/`):
//   ""           → Consumer app simulation (buy → sign up → add device → ready)
//   "app"        → same consumer onboarding flow (explicit)
//   "dashboard"  → Devices dashboard (the fleet overview, reads the DB)
//   "provision"  → Provision a device (the setup-automation page)
//
// The sidebar nav (`nav.tsx`) links to all three.

import * as React from "react";
import { useSlotContext } from "@nube/starter-ext-sdk-ts";

import ConsumerOnboard from "./onboard";
import DevicesDashboard from "./dashboard";
import DevicesPanel from "./panel";

export default function Main(): React.ReactElement {
  const { route } = useSlotContext();
  // Normalise: strip any leading slash and take the first path segment.
  const head = (route ?? "").replace(/^\/+/, "").split("/")[0];

  switch (head) {
    case "provision":
      return <DevicesPanel />;
    case "dashboard":
      return <DevicesDashboard />;
    case "":
    case "app":
    default:
      return <ConsumerOnboard />;
  }
}
