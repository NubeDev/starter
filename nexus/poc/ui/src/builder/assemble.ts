// Assemble a StreamConfig from the builder's selected components + field values.

import type { ComponentKind } from "../api/catalog";
import type { StreamConfig } from "../api/streams";
import { coerceField } from "./coerce";

/** A picked component: its catalog entry plus the raw form values keyed by field. */
export interface Picked {
  kind: ComponentKind;
  values: Record<string, string>;
}

function configFrom(picked: Picked): Record<string, unknown> {
  const out: Record<string, unknown> = { type: picked.kind.type };
  for (const field of picked.kind.fields) {
    const v = coerceField(field.kind, picked.values[field.name] ?? "");
    if (v !== undefined) out[field.name] = v;
  }
  return out;
}

export function assembleConfig(args: {
  input: Picked;
  buffer: Picked | null;
  processors: Picked[];
  output: Picked;
}): StreamConfig {
  const config: StreamConfig = {
    input: configFrom(args.input) as StreamConfig["input"],
    pipeline: {
      thread_num: 1,
      processors: args.processors.map(configFrom) as StreamConfig["pipeline"]["processors"],
    },
    output: configFrom(args.output) as StreamConfig["output"],
  };
  if (args.buffer) config.buffer = configFrom(args.buffer) as StreamConfig["buffer"];
  return config;
}
