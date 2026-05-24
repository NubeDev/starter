// `RubixClient` — the long-lived handle endpoint modules hang
// methods off via TypeScript declaration-merging, mirroring
// `StarterClient` from `@nube/starter-client-ts`.
//
// A `RubixClient` is constructed from an existing `StarterClient` so
// transport configuration (baseUrl, fetch override, default headers,
// CSRF cookies) lives in exactly one place. The wrapped starter is
// exposed read-only as `.starter` so endpoint helpers can reach the
// shared fetch + baseUrl + headers without re-plumbing them.

import { StarterClient } from "@nube/starter-client-ts";

export class RubixClient {
  readonly starter: StarterClient;

  constructor(starter: StarterClient) {
    this.starter = starter;
  }
}
