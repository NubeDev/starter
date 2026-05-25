// `SduiTransport` — the single seam between this package and any I/O.
//
// The package never imports `fetch` directly, never reads
// `process.env`, never hard-codes a URL. Every hook (`useSduiResolve`,
// `useSduiAction`, `useSduiSubscriptions`) reads its transport via
// `<SduiProvider>`'s context. Rubix supplies a concrete impl that
// rides on `StarterClient`; tests supply an in-memory fixture.

import type { StarterClient } from "@nube/starter-client-ts";
import type {
  ActionRequest,
  ResolveRequest,
  SubscriptionSubject,
  TableRequest,
  TableResponse,
  UiActionResponse,
  UiResolveResponse,
} from "@nube/starter-ui-ir";

export interface SduiTransport {
  resolve(req: ResolveRequest, signal?: AbortSignal): Promise<UiResolveResponse>;
  action(req: ActionRequest, signal?: AbortSignal): Promise<UiActionResponse>;
  table(req: TableRequest, signal?: AbortSignal): Promise<TableResponse>;
  /**
   * Open a subscription to the given subjects. The implementation
   * MAY use SSE, websockets, or polling; the renderer only needs
   * the per-subject callback + an unsubscribe handle.
   */
  subscribe(
    subjects: SubscriptionSubject[],
    onUpdate: (subj: SubscriptionSubject, value: unknown) => void,
  ): () => void;
}

export interface HttpSduiTransportOptions {
  /** The shared `StarterClient` instance. */
  client: StarterClient;
  /**
   * API path prefix used for SDUI routes. Defaults to `/ui` so the
   * effective base becomes `<client.apiPrefix>/ui/...`.
   */
  pathPrefix?: string;
  /**
   * Polling interval (ms) used by the subscription fallback when
   * SSE isn't wired up server-side. Defaults to 15_000.
   */
  pollIntervalMs?: number;
}

interface FetchInput {
  method: "POST";
  url: string;
  body: unknown;
  signal?: AbortSignal;
}

/**
 * Concrete `SduiTransport` that rides on `StarterClient`. Mirrors
 * `packages/starter-ui-ai-builder/src/adapters/http.ts` in spirit:
 * one HTTP fn, typed JSON in/out, no other I/O.
 */
export function createHttpSduiTransport(
  opts: HttpSduiTransportOptions,
): SduiTransport {
  const { client } = opts;
  const prefix = (opts.pathPrefix ?? "/ui").replace(/\/$/, "");
  const interval = Math.max(1_000, opts.pollIntervalMs ?? 15_000);

  async function postJson<T>(input: FetchInput): Promise<T> {
    const response = await client.fetch(
      `${client.baseUrl}${client.apiPrefix}${input.url}`,
      {
        method: input.method,
        headers: {
          "content-type": "application/json",
          ...client.headers,
        },
        body: JSON.stringify(input.body),
        signal: input.signal,
      },
    );
    if (!response.ok) {
      const hint = await response.text().catch(() => "");
      throw new Error(`${response.status} ${response.statusText}: ${hint}`);
    }
    return (await response.json()) as T;
  }

  return {
    resolve(req, signal) {
      return postJson<UiResolveResponse>({
        method: "POST",
        url: `${prefix}/resolve`,
        body: req,
        signal,
      });
    },
    action(req, signal) {
      return postJson<UiActionResponse>({
        method: "POST",
        url: `${prefix}/action`,
        body: req,
        signal,
      });
    },
    table(req, signal) {
      return postJson<TableResponse>({
        method: "POST",
        url: `${prefix}/table`,
        body: req,
        signal,
      });
    },
    subscribe(subjects, onUpdate) {
      // Default to a polling loop. Rubix overrides this with an SSE
      // impl once the backend ships `GET /ui/subscribe`.
      let cancelled = false;
      const tick = () => {
        if (cancelled) return;
        for (const subj of subjects) onUpdate(subj, undefined);
      };
      const handle = setInterval(tick, interval);
      return () => {
        cancelled = true;
        clearInterval(handle);
      };
    },
  };
}
