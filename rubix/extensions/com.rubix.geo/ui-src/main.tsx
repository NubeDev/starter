// TODO: implement with shadcn/ui + MapLibre GL
import * as React from "react";
import "./app.css";
import { BlockShell } from "@nube/starter-ext-sdk-ts";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <div className="p-4">
        <h3 className="text-lg font-semibold">Rubix Geo</h3>
        <p className="text-sm text-muted-foreground">Map view — TODO</p>
      </div>
    </BlockShell>
  );
}
