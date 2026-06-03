// `main.tsx` — Main panel for `com.nubeio.ce`.
//
// Two routes under the extension:
//
//   /extensions/com.nubeio.ce            → device list (CRUD)
//   /extensions/com.nubeio.ce/devices    → device list (CRUD)
//   /extensions/com.nubeio.ce/wiresheet/<device_id>
//                                        → wiresheet editor for a device
//
// All data flows through `com.nubeio.ce.warehouse_query` and the
// `device_*` / `engine_*` tools (see ce-api.ts). This is a scaffold:
// the panels render the chrome + call surface, no business logic.

import * as React from "react";
import "./app.css";

import { BlockShell, useExtensionRoute } from "@nube/starter-ext-sdk-ts";

declare const __BUILD_STAMP__: string;
if (typeof window !== "undefined") {
  // eslint-disable-next-line no-console
  console.info("[com.nubeio.ce] bundle loaded — build-", __BUILD_STAMP__);
}

import { Page } from "./ui/page";
import { DevicesPanel } from "./devices/devices-panel";
import { WiresheetPanel } from "./wiresheet/wiresheet-panel";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainRouter />
    </BlockShell>
  );
}

function MainRouter(): React.ReactElement {
  const route = useExtensionRoute();

  // /wiresheet/<device_id> — full-bleed editor, no Page chrome. The panel
  // renders a fixed full-viewport surface so the lifted ce-ui editor and its
  // viewport-anchored floating panels line up (matches the standalone app).
  if (route && route.startsWith("wiresheet")) {
    const deviceId = route.split("/")[1] ?? "";
    return <WiresheetPanel deviceId={deviceId} />;
  }

  // default + /devices → device catalog
  return (
    <Page eyebrow="Control Engine" title="Devices">
      <DevicesPanel />
    </Page>
  );
}
