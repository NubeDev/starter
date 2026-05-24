// Error type the client throws when the server returns a `Problem`
// body or transport fails.

import type { components } from "../generated/index.js";

export type Problem = components["schemas"]["Problem"];

export class StarterError extends Error {
  readonly status: number;
  readonly problem: Problem | undefined;

  constructor(status: number, message: string, problem?: Problem) {
    super(message);
    this.name = "StarterError";
    this.status = status;
    this.problem = problem;
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

  // Type guard. With one arg, narrows to StarterError; with two, also
  // requires that `.status` matches.
  static is(err: unknown, status?: number): err is StarterError {
    if (!(err instanceof StarterError)) return false;
    return status === undefined || err.status === status;
  }
}
