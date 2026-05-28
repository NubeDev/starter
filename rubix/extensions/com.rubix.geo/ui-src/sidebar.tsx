// TODO: implement with shadcn/ui
import * as React from "react";
import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";

export default function Sidebar(): React.ReactElement {
  return (
    <BlockShell>
      <div className="mx-2 my-1 py-2 px-3 rounded-md border border-border">
        <div className="text-xs font-medium">Rubix Geo</div>
        <a
          href={`/extensions/${EXTENSION_ID}`}
          className="text-xs text-primary hover:underline mt-1 inline-block"
        >
          open map →
        </a>
      </div>
    </BlockShell>
  );
}
