// `GET /openapi.json` — fetched by the codegen workflow to (re-)build
// the rest of this package's types.

import { StarterClient } from "../client/client.js";

declare module "../client/client.js" {
  interface StarterClient {
    openapi(): Promise<unknown>;
  }
}

StarterClient.prototype.openapi = async function openapi(this: StarterClient): Promise<unknown> {
  const res = await this.fetch(`${this.baseUrl}/openapi.json`, { headers: this.headers });
  return await res.json();
};
