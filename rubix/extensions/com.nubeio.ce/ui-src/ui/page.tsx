// `ui/page.tsx` — page chrome shared by every route.
//
// Wraps content in the `data-ext-id` div the CSS-scoping plugin keys
// off of (see vite.config.ts / app.css), plus a header with an eyebrow
// + title. Every extension page must carry the `data-ext-id` attribute
// or its Tailwind utilities won't apply.

import * as React from "react";

import { useSlotContext } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "../types";

export function Page({
  eyebrow,
  title,
  actions,
  children,
}: {
  eyebrow?: string;
  title: string;
  actions?: React.ReactNode;
  children: React.ReactNode;
}): React.ReactElement {
  const slot = useSlotContext();
  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      className="flex flex-col gap-4 p-4"
    >
      <header className="flex items-end justify-between gap-3">
        <div className="flex flex-col gap-0.5">
          {eyebrow ? <span className="ext-eyebrow">{eyebrow}</span> : null}
          <h3 className="text-xl font-semibold">{title}</h3>
        </div>
        {actions ? <div className="flex items-center gap-2">{actions}</div> : null}
      </header>
      {children}
    </div>
  );
}
