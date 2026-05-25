// Error type the client throws when the server returns a `Problem`
// body or transport fails.

import type { components } from "../generated/index.js";

export type Problem = components["schemas"]["Problem"];

export class StarterError extends Error {
  readonly status: number;
  readonly problem: Problem | undefined;
  /**
   * Machine-readable error code. Set by client-side factories
   * (`invalidResponse` etc.) for cases the server cannot tag
   * itself. Server-driven errors carry their tag in `problem.type`.
   */
  readonly code: string | undefined;

  constructor(status: number, message: string, problem?: Problem, code?: string) {
    super(message);
    this.name = "StarterError";
    this.status = status;
    this.problem = problem;
    this.code = code;
  }

  static async fromResponse(res: Response): Promise<StarterError> {
    let problem: Problem | undefined;
    try {
      const body = (await res.clone().json()) as Problem;
      if (body && typeof body === "object" && "type" in body && "title" in body) {
        problem = body;
      }
    } catch {
      // not JSON or not a Problem — fall through.
    }
    const msg = problem?.title ?? `HTTP ${res.status}`;
    return new StarterError(res.status, msg, problem);
  }

  /**
   * Build an error for a 2xx response whose body is not JSON.
   * Typical cause: a dev-server SPA fallback returned `index.html`
   * instead of forwarding the request to the API — meaning the
   * client is asking a path the proxy does not cover. Surfaced as
   * `status = 502` + `code = "invalid-response-content-type"` so
   * callers (notably `AuthProvider`) can distinguish it from a
   * genuine server error.
   */
  static invalidResponse(url: string, contentType: string | null): StarterError {
    const ct = contentType ?? "<none>";
    return new StarterError(
      502,
      `Expected JSON from ${url} but got content-type ${ct}. ` +
        `This usually means the request was not routed to the API (e.g. the dev-server proxy is missing this path).`,
      undefined,
      "invalid-response-content-type",
    );
  }

  // Type guard. With one arg, narrows to StarterError; with two, also
  // requires that `.status` matches.
  static is(err: unknown, status?: number): err is StarterError {
    if (!(err instanceof StarterError)) return false;
    return status === undefined || err.status === status;
  }
}
