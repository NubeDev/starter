import type { UiComponentTree } from "@nube/starter-sdui-react";
import type { BuilderAdapter, BuilderEvent } from "../types/index.js";

export interface FixtureBuilderAdapterOptions {
  /**
   * Map of prompt-prefix → scripted event stream. The adapter picks
   * the first prefix that matches `input.text` (case-insensitive); if
   * none match, the `default` script runs.
   */
  scripts: Record<string, BuilderEvent[]>;
  /** Delay between yielded events (ms). Default: 150. */
  delayMs?: number;
}

/**
 * Reference adapter for demos and tests. Replays canned
 * `BuilderEvent` sequences keyed off the prompt. Honours `signal` —
 * cancelling mid-script stops the stream.
 */
export function createFixtureBuilderAdapter(
  opts: FixtureBuilderAdapterOptions,
): BuilderAdapter {
  const delay = opts.delayMs ?? 150;
  return {
    async *send(input, signal) {
      const q = input.text.trim().toLowerCase();
      let script: BuilderEvent[] | undefined;
      for (const [prefix, events] of Object.entries(opts.scripts)) {
        if (prefix === "default") continue;
        if (q.startsWith(prefix.toLowerCase())) {
          script = events;
          break;
        }
      }
      script = script ?? opts.scripts.default ?? [];
      for (const ev of script) {
        if (signal.aborted) return;
        await new Promise<void>((resolve, reject) => {
          if (!delay) return resolve();
          const t = setTimeout(resolve, delay);
          signal.addEventListener(
            "abort",
            () => {
              clearTimeout(t);
              reject(signal.reason ?? new DOMException("aborted", "AbortError"));
            },
            { once: true },
          );
        }).catch(() => {});
        if (signal.aborted) return;
        yield ev;
      }
    },
  };
}

/**
 * Tiny helper to author fixture trees without typing the IR envelope
 * boilerplate every time.
 */
export function fixtureTree(root: UiComponentTree["root"], irVersion = 5): UiComponentTree {
  return { ir_version: irVersion, root };
}
