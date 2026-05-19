// Error type the client throws when the server returns a `Problem`
// body or transport fails.

import type { Problem } from "../generated/index.js";

export class StarterError extends Error {
  readonly status: number;
  readonly problem: Problem | undefined;

  constructor(status: number, message: string, problem?: Problem) {
    super(message);
    this.name = "StarterError";
    this.status = status;
    this.problem = problem;
  }
}
