// The one component this extension exposes — a tiny sidebar panel.
//
// `BlockShell` wraps the panel content with the standard error
// boundary + loading skeleton (`@nube/starter-ext-sdk-ts`).
// `useSlotContext` reads the slot id and host theme.
//
// The panel does not issue raw `fetch` calls (SCOPE R11); a real
// extension that needs server data uses `useHostClient()` from the
// SDK. This minimal example only renders text, so the demo is left
// as a comment.

import * as React from "react";

import { BlockShell, useSlotContext } from "@nube/starter-ext-sdk-ts";

export default function HelloPanel(): React.ReactElement {
  return (
    <BlockShell>
      <HelloPanelInner />
    </BlockShell>
  );
}

function HelloPanelInner(): React.ReactElement {
  const slot = useSlotContext();
  return (
    <section className="hello-ui-panel">
      <h3>Hello from {slot.extensionId}</h3>
      <p>
        Mounted in <code>{slot.slotId}</code> (theme: <em>{slot.theme}</em>).
      </p>
    </section>
  );
}
