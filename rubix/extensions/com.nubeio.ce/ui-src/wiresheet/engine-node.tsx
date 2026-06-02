// `wiresheet/engine-node.tsx` — the visual for every control-engine
// block, built on the package's `BaseNode`.
//
// `FlowCanvas` populates `data.kindSpec` + `data.label` on each
// @xyflow/react node before rendering, so this thin wrapper just
// forwards them into BaseNode. All four wiresheet kinds share this
// component (they differ only by spec); swap in per-kind bodies via
// `children` if a block needs a custom config preview.

import * as React from "react";

import type { NodeProps } from "@xyflow/react";
import { BaseNode } from "@nube/starter-ui-flow/nodes";
import type { NodeKindSpec } from "@nube/starter-ui-flow";

export function EngineNode(props: NodeProps): React.ReactElement {
  const data = props.data as {
    kindSpec: NodeKindSpec;
    label?: string;
  };
  return (
    <BaseNode
      spec={data.kindSpec}
      label={data.label}
      selected={props.selected}
      variant="full"
    />
  );
}
