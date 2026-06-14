import type { CreateFlowRequest } from "@/api/types";

// A flow's input / pipeline / output are opaque ArkFlow config the backend
// stores verbatim (typed as free-form JSON in the contract). The editor
// authors each section as text; this module parses + validates that text
// before it reaches the API, so a typo surfaces in the form, not as a 400.

export type ParseResult =
  | { ok: true; value: unknown }
  | { ok: false; error: string };

// Parse one config section. Blank → an empty object (the section is
// omitted). A valid object or array passes (inputs/outputs are objects, a
// pipeline is a list of processors); a bare scalar/null is rejected — a
// flow section is always a structured value.
export function parseFlowSection(text: string): ParseResult {
  const trimmed = text.trim();
  if (trimmed === "") return { ok: true, value: {} };
  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : "Invalid JSON" };
  }
  if (typeof parsed !== "object" || parsed === null) {
    return { ok: false, error: "Must be a JSON object or array" };
  }
  return { ok: true, value: parsed };
}

export interface FlowDraft {
  name: string;
  enabled: boolean;
  input: string;
  pipeline: string;
  output: string;
}

export type FlowBuildResult =
  | { ok: true; value: CreateFlowRequest }
  | { ok: false; field: "input" | "pipeline" | "output"; error: string };

// Assemble a create request from the draft, reporting *which* section
// failed so the form can mark the right editor. The three config sections
// are parsed independently; the first failure wins.
export function toCreateFlow(draft: FlowDraft): FlowBuildResult {
  const sections: Array<["input" | "pipeline" | "output", string]> = [
    ["input", draft.input],
    ["pipeline", draft.pipeline],
    ["output", draft.output],
  ];
  const parsed: Record<string, unknown> = {};
  for (const [field, text] of sections) {
    const r = parseFlowSection(text);
    if (!r.ok) return { ok: false, field, error: r.error };
    parsed[field] = r.value;
  }
  return {
    ok: true,
    value: {
      name: draft.name.trim(),
      enabled: draft.enabled,
      input: parsed.input,
      pipeline: parsed.pipeline,
      output: parsed.output,
    },
  };
}
