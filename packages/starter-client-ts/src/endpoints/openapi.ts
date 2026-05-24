// `GET /openapi.json` — fetched by the codegen workflow to (re-)build
// the rest of this package's types.

import { StarterClient } from "../client/client.js";
import { fetchJson } from "../client/fetch_json.js";

declare module "../client/client.js" {
  interface StarterClient {
    openapi(): Promise<unknown>;
  }
}

StarterClient.prototype.openapi = function openapi(this: StarterClient): Promise<unknown> {
  return fetchJson<unknown>(this, `/openapi.json`);
};
