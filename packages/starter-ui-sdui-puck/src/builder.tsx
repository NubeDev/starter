// PuckBuilder — stub mounting `<Puck>` with the generated Config.
// PR1 wires the canvas and palette so the editor can be inspected
// via the package's harness; B4 (save path) and B5 (route) land in
// later PRs, which is why `onChange` is currently a no-op.

import { Puck, type Config, type Data } from "@measured/puck";
import { useMemo, type ReactElement } from "react";

import { buildPuckConfig } from "./build-puck-config.js";
import { IR_SCHEMA } from "./schema-loader.js";
import type { PuckConfigStub } from "./puck-types.js";

export interface PuckBuilderProps {
  /** Page identifier — `"dashboard.<slug>"`. Echoed in B4 save call. */
  pageRef: string;
  /** Initial Puck `Data` (root + content). PR1 accepts Puck's shape
   *  directly; PR2 wires the IR ComponentTree ↔ Puck Data adapter
   *  alongside the save path. */
  initialData?: Data;
  /** Optional pre-built config — defaults to the schema-derived one. */
  config?: Config;
}

export function PuckBuilder({
  pageRef,
  initialData,
  config,
}: PuckBuilderProps): ReactElement {
  const resolvedConfig = useMemo<Config>(() => {
    if (config) return config;
    // Cast the structural stub into Puck's typed Config: the stub
    // shape is a subset of Puck's union by construction (see
    // puck-types.ts).
    return buildPuckConfig({ schema: IR_SCHEMA }) as unknown as Config;
  }, [config]);

  const data = initialData ?? { content: [], root: { props: {} } };

  return (
    <Puck
      config={resolvedConfig}
      data={data}
      onChange={(next) => {
        // PR1 stub: PR2 lands save/onSave wiring to
        // rubix.dashboard.update via the standard REST envelope.
        // For now we just expose the latest Data on `window` so the
        // harness can poke at it without crashing.
        if (typeof window !== "undefined") {
          (window as unknown as Record<string, unknown>).__rubixPuckLastChange = {
            pageRef,
            data: next,
            ts: Date.now(),
          };
        }
      }}
    />
  );
}

// Re-export the structural config type so harness code can type the
// build output without depending on the @measured/puck module path.
export type { PuckConfigStub };
