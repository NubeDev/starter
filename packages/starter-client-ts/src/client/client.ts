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
}

export class StarterClient {
  readonly baseUrl: string;
  readonly fetch: typeof fetch;
  readonly headers: Record<string, string>;

  constructor(opts: StarterClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, "");
    this.fetch = opts.fetch ?? globalThis.fetch.bind(globalThis);
    this.headers = opts.headers ?? {};
  }
}
