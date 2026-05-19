// `GET /health` client method.

import { StarterClient } from "../client/client.js";

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

StarterClient.prototype.health = async function health(this: StarterClient): Promise<Health> {
  const res = await this.fetch(`${this.baseUrl}/health`, { headers: this.headers });
  return (await res.json()) as Health;
};
