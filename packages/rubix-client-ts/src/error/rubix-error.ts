// `RubixError` extends `StarterError` and adds the rubix-specific
// `code` field carried by the agent's Diagnostic envelope.
//
// Wire shape (per SCOPE OQ-4): the rubix-agent typically returns a
// Diagnostic-shaped body whose top-level `summary` object carries the
// machine-readable `code` (e.g. `"rubix.system.disk"`). Some legacy
// verbs put `code` at the body root, so we accept either:
//
//   { "summary": { "code": "rubix.foo.bar", ... }, ... }   ← preferred
//   { "code": "rubix.foo.bar", ... }                       ← fallback
//
// `.code` is `undefined` if neither is present (transport error,
// non-JSON body, or a verb that returns a bare `Problem`).

import { StarterError } from "@nube/starter-client-ts";
import type { Problem } from "@nube/starter-client-ts";

interface RubixBody {
  code?: unknown;
  summary?: { code?: unknown } | null;
}

function extractCode(body: unknown): string | undefined {
  if (!body || typeof body !== "object") return undefined;
  const b = body as RubixBody;
  const summaryCode = b.summary?.code;
  if (typeof summaryCode === "string") return summaryCode;
  if (typeof b.code === "string") return b.code;
  return undefined;
}

export class RubixError extends StarterError {
  readonly code: string | undefined;

  constructor(
    status: number,
    message: string,
    problem?: Problem,
    code?: string,
  ) {
    super(status, message, problem);
    this.name = "RubixError";
    this.code = code;
  }

  static override async fromResponse(res: Response): Promise<RubixError> {
    // Parse the body once for code extraction, then defer to
    // `StarterError.fromResponse` for the Problem-shape branch so we
    // don't duplicate its logic.
    let raw: unknown;
    try {
      raw = await res.clone().json();
    } catch {
      raw = undefined;
    }
    const code = extractCode(raw);
    const parent = await StarterError.fromResponse(res);
    return new RubixError(parent.status, parent.message, parent.problem, code);
  }
}
