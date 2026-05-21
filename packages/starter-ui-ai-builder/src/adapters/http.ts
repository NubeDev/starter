// HTTP/SSE BuilderAdapter — talks to `POST /api/builder/stream`.
//
// Contract pinned by `examples/flow-agent/PAGE-BUILDER-LIVE.md` §6
// and `…-FRONTEND.md` §4.1. The wire shape (one JSON `BuilderEvent`
// per SSE `data:` frame) is the **only** seam shared with the
// backend session; do not invent new fields here.

import type { BuilderAdapter, BuilderEvent } from "../types/index.js";

export interface HttpBuilderAdapterOptions {
  /** Backend SSE endpoint. */
  url: string;
  /**
   * Optional silent-fallback factory invoked on HTTP 503. When set,
   * the adapter substitutes the returned `BuilderAdapter` for THIS
   * send call only — do not cache across calls, the user may bring
   * the backend back. When unset, a 503 surfaces a single
   * `{ type: "error", error: <hint> }` frame instead.
   */
  onUnavailable?: () => BuilderAdapter;
  /**
   * Optional fetch override — used by unit tests to drive an
   * in-memory `ReadableStream` instead of the real network. Defaults
   * to `globalThis.fetch`.
   */
  fetch?: typeof fetch;
}

export function createHttpBuilderAdapter(
  opts: HttpBuilderAdapterOptions,
): BuilderAdapter {
  const doFetch = opts.fetch ?? ((...args) => fetch(...args));
  return {
    async *send(input, signal) {
      if (signal.aborted) return;

      let response: Response;
      try {
        // Build the body conditionally so callers that don't opt
        // into session persistence keep the historical wire shape
        // (`{ prompt, provider }`) — the backend treats omitted
        // `session_id` as ephemeral (MEMORY.md M13).
        const body: {
          prompt: string;
          provider: string;
          mode?: string;
          session_id?: string;
          include_artifact?: string;
        } = {
          prompt: input.text,
          provider: "claude",
        };
        if (input.mode) {
          body.mode = input.mode;
        }
        if (input.sessionId) {
          body.session_id = input.sessionId;
          if (input.includeArtifact) {
            body.include_artifact = input.includeArtifact;
          }
        }
        response = await doFetch(opts.url, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
          signal,
        });
      } catch (err) {
        if (signal.aborted) return;
        yield {
          type: "error",
          error: `network error: ${errorMessage(err)}`,
        };
        return;
      }

      if (signal.aborted) return;

      if (response.status === 503 && opts.onUnavailable) {
        // Drain & discard the 503 body so the fallback isn't racing
        // a still-open socket on slow networks.
        try {
          await response.body?.cancel();
        } catch {
          /* ignore */
        }
        const fallback = opts.onUnavailable();
        for await (const ev of fallback.send(input, signal)) {
          if (signal.aborted) return;
          yield ev;
        }
        return;
      }

      if (!response.ok) {
        const hint = await readErrorHint(response);
        yield { type: "error", error: hint };
        return;
      }

      const body = response.body;
      if (!body) {
        yield { type: "error", error: "empty response body" };
        return;
      }

      const reader = body.getReader();
      const decoder = new TextDecoder("utf-8");
      let buffer = "";

      try {
        while (true) {
          if (signal.aborted) {
            await safeCancel(reader);
            return;
          }

          let chunk: ReadableStreamReadResult<Uint8Array>;
          try {
            chunk = await reader.read();
          } catch (err) {
            if (signal.aborted) return;
            yield {
              type: "error",
              error: `stream read failed: ${errorMessage(err)}`,
            };
            return;
          }
          if (chunk.done) {
            // Flush any trailing frame without the terminator.
            const tail = buffer.trim();
            buffer = "";
            if (tail.length > 0) {
              const ev = parseFrame(tail);
              if (ev && !signal.aborted) yield ev;
            }
            return;
          }
          buffer += decoder.decode(chunk.value, { stream: true });

          // Standard SSE framing: events end at "\n\n". Be tolerant
          // of CRLF too in case a proxy rewrites line endings.
          let sepIdx: number;
          while ((sepIdx = findFrameSeparator(buffer)) !== -1) {
            const rawFrame = buffer.slice(0, sepIdx);
            buffer = buffer.slice(sepIdx + frameSeparatorLength(buffer, sepIdx));
            const ev = parseFrame(rawFrame);
            if (signal.aborted) return;
            if (ev) yield ev;
          }
        }
      } finally {
        await safeCancel(reader);
      }
    },
  };
}

/**
 * Parse one SSE frame body (everything before the blank-line
 * separator) into a `BuilderEvent`. Returns `null` for frames that
 * are intentionally ignorable (comments, `[DONE]`, empty data).
 * Returns a synthetic error event for frames whose `data:` payload
 * fails `JSON.parse`.
 */
function parseFrame(rawFrame: string): BuilderEvent | null {
  // Concatenate multi-line `data:` payloads with "\n" per the SSE
  // spec; ignore comments and unknown fields (event:, id:, retry:).
  const dataLines: string[] = [];
  for (const lineRaw of rawFrame.split(/\r?\n/)) {
    const line = lineRaw;
    if (line.length === 0) continue;
    if (line.startsWith(":")) continue; // comment
    if (line.startsWith("data:")) {
      // Per spec: a single optional space after the colon is stripped.
      const payload = line.slice(5).replace(/^ /, "");
      dataLines.push(payload);
    }
    // event:/id:/retry: are not used by this contract; ignore silently.
  }
  if (dataLines.length === 0) return null;
  const payload = dataLines.join("\n");
  // Chat-surface convention; never terminal for the builder.
  if (payload === "[DONE]") return null;
  try {
    return JSON.parse(payload) as BuilderEvent;
  } catch {
    return {
      type: "error",
      error: `malformed sse frame: ${preview(payload)}`,
    };
  }
}

function findFrameSeparator(buf: string): number {
  const lf = buf.indexOf("\n\n");
  const crlf = buf.indexOf("\r\n\r\n");
  if (lf === -1) return crlf;
  if (crlf === -1) return lf;
  return Math.min(lf, crlf);
}

function frameSeparatorLength(buf: string, idx: number): number {
  return buf.startsWith("\r\n\r\n", idx) ? 4 : 2;
}

async function readErrorHint(response: Response): Promise<string> {
  let bodyText = "";
  try {
    bodyText = (await response.text()).trim();
  } catch {
    /* ignore */
  }
  if (bodyText) {
    // Try to lift a structured `{ "error": "..." , "hint": "..." }`
    // body; fall back to the raw text otherwise.
    try {
      const parsed = JSON.parse(bodyText) as {
        error?: unknown;
        hint?: unknown;
      };
      const err = typeof parsed.error === "string" ? parsed.error : "";
      const hint = typeof parsed.hint === "string" ? parsed.hint : "";
      const joined = [err, hint].filter(Boolean).join(" — ");
      if (joined) return joined;
    } catch {
      // not JSON, fall through
    }
    return preview(bodyText);
  }
  return `${response.status} ${response.statusText || "error"}`.trim();
}

async function safeCancel(
  reader: ReadableStreamDefaultReader<Uint8Array>,
): Promise<void> {
  try {
    await reader.cancel();
  } catch {
    /* ignore — reader may already be closed */
  }
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

function preview(s: string): string {
  const trimmed = s.trim();
  return trimmed.length <= 200 ? trimmed : `${trimmed.slice(0, 200)}…`;
}
