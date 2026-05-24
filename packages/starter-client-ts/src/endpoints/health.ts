// `GET /health` client method.

import { StarterClient } from "../client/client.js";
import { fetchJson } from "../client/fetch_json.js";

/** Health-check response. Mirror of `starter_spi::dto::Health`. */
export interface Health {
  status: string;
  version: string;
  uptime_seconds: number;
}

declare module "../client/client.js" {
  interface StarterClient {
    health(): Promise<Health>;
  }
}

StarterClient.prototype.health = function health(this: StarterClient): Promise<Health> {
  return fetchJson<Health>(this, `/health`);
};
