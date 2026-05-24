// `ui/main.tsx` — minimal UI contribution for com.rubix.example.
//
// SCOPE Phase D notes that this reference extension previously
// shipped no UI assets (per OQ-3 evidence at the Phase B gate). The
// frontend host-provider page in `packages/test-ui-5` needs at least
// one extension to render something visible into its `main` slot, so
// this file exists to fulfil that contract.
//
// The component is intentionally trivial: it reads the host theme via
// `useHostTheme()` (mirroring host-context theming per SCOPE R11) and
// renders a single `hello-from-com.rubix.example` line stamped with
// the resolved theme mode so the visual check confirms theme tokens
// reach the extension surface.

import * as React from "react";

import { BlockShell, useSlotContext, useHostTheme } from "@nube/starter-ext-sdk-ts";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainInner />
    </BlockShell>
  );
}

function MainInner(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();
  return (
    <section
      data-ext-id="com.rubix.example"
      data-ext-slot={slot.slotId}
      data-ext-theme={theme.mode}
      style={{
        padding: "0.75rem 1rem",
        borderRadius: "0.5rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
      }}
    >
      <p style={{ margin: 0 }}>hello-from-com.rubix.example</p>
      <small style={{ opacity: 0.7 }}>
        slot=<code>{slot.slotId}</code> · theme=<code>{theme.mode}</code>
      </small>
    </section>
  );
}
