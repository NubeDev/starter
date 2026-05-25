// `StarterClient` — the long-lived handle endpoint modules hang
// methods off via TypeScript declaration-merging.
//
// Construction is intentionally simple: pass a base URL and optional
// fetch override (so tests can inject `msw` or a custom transport).

export interface StarterClientOptions {
  /** Server base URL, e.g. `http://localhost:8080`. */
  baseUrl: string;
  /** Override the global `fetch`. Defaults to `globalThis.fetch`. */
  fetch?: typeof fetch;
  /** Default headers attached to every request. */
  headers?: Record<string, string>;
  /**
   * API path prefix prepended to every endpoint URL.
   *
   * Defaults to `/api/v1` so the starter contract — `/api/v1/auth/login`,
   * `/api/v1/ui/theme`, etc. — works out of the box. Pass an empty string
   * `""` (or `"/"`) to talk to a server that mounts the same routes at the
   * root (e.g. dev-pulse's dp-server exposes `/auth/login` directly).
   *
   * Leading slash is normalised; trailing slash is stripped.
   */
  apiPrefix?: string;
}

export class StarterClient {
  readonly baseUrl: string;
  readonly fetch: typeof fetch;
  readonly headers: Record<string, string>;
  /** Normalised API prefix — either `""` or `/<segment>...` with no
   *  trailing slash. Endpoint modules read this to build paths. */
  readonly apiPrefix: string;

  constructor(opts: StarterClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, "");
    this.fetch = opts.fetch ?? globalThis.fetch.bind(globalThis);
    this.headers = opts.headers ?? {};
    this.apiPrefix = normalisePrefix(opts.apiPrefix ?? "/api/v1");
  }
}

/** Normalise a user-supplied prefix to `""` or `/<segment>...`.
 *
 *  - `""` and `"/"` → `""` (server mounts routes at the root).
 *  - `"api/v1"` → `"/api/v1"` (leading slash added).
 *  - `"/api/v1/"` → `"/api/v1"` (trailing slash stripped).
 */
function normalisePrefix(prefix: string): string {
  const trimmed = prefix.replace(/\/+$/, "");
  if (trimmed === "") return "";
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}
