import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { replaceAt, type UiComponentTree } from "@nube/starter-sdui-react";
import type {
  BuilderAdapter,
  BuilderEvent,
  BuilderMode,
  BuilderPhase,
  BuilderSendInput,
  ShellPatch,
  TokenPatch,
} from "../types/index.js";
import { makeId, treeHasId } from "../lib/utils.js";

/** A user prompt, status frame, or assistant prose reply (Ask
 *  mode), in chronological order, for the transcript pane. The
 *  library never invents AI bubbles for Build turns — assistant
 *  output IS the canvas there. Ask turns are the only path that
 *  populates an `"assistant"` entry. */
export interface BuilderTranscriptEntry {
  id: string;
  kind: "user" | "assistant" | "status";
  text: string;
  phase?: BuilderPhase;
  /** Set on `kind: "user"` entries to indicate which lane the
   *  prompt was sent on. UI surfaces use this to colour the
   *  bubble (e.g. a subtle "Ask" tag). */
  mode?: BuilderMode;
  createdAt: number;
}

export interface UseBuilderOptions {
  adapter: BuilderAdapter;
  /** Initial tree to show before the first stream lands. */
  initialTree?: UiComponentTree | null;
  /** Initial conversation lane. Defaults to `"build"`. The hook
   *  also exposes `mode` + `setMode` so the surface can toggle. */
  defaultMode?: BuilderMode;
  /** R1 — buffer `patch` events whose target isn't in the tree yet
   *  for this many ms. Default: 2000 (the SCOPE-mandated window). */
  patchBufferMs?: number;
  /** Optional sink for theme-slice payloads. The page-builder slice
   *  ignores these by default. */
  onTokenPatch?: (patch: TokenPatch) => void;
  onShellPatch?: (patch: ShellPatch) => void;
  /** MEMORY.md Phase M-D — invoked when the server confirms the
   *  assistant turn was persisted as a versioned artifact. Surfaces
   *  use this to refresh a version picker / undo state. */
  onSessionArtifact?: (info: {
    sessionId: string;
    key: string;
    version?: number;
  }) => void;
  /** MEMORY.md Phase M-D — invoked when the server completed the
   *  turn but the session-store write failed. The response is still
   *  valid; surfaces should degrade gracefully (stay stateless). */
  onSessionError?: (error: string) => void;
  onError?: (err: unknown) => void;
}

export interface UseBuilderReturn {
  tree: UiComponentTree | null;
  transcript: BuilderTranscriptEntry[];
  phase: BuilderPhase;
  error: string | null;
  /** Current conversation lane. Used by the composer to pick the
   *  Build/Ask toggle state. */
  mode: BuilderMode;
  setMode: (mode: BuilderMode) => void;
  send: (input: BuilderSendInput | string) => Promise<void>;
  cancel: () => void;
  /** Re-run the last user prompt (drops the trailing transcript
   *  status frames). No-op if there's no prior prompt. */
  retry: () => Promise<void>;
  reset: () => void;
  /** MEMORY.md Phase M-D — imperatively replace the canvas tree
   *  outside a streaming turn. Surfaces use this to hydrate from a
   *  persisted artifact on mount, or to jump to a historical
   *  version via the artifact-versions endpoint. Does not record
   *  anything — the store sees only what the model writes. */
  setTree: (tree: UiComponentTree | null) => void;
  /** Imperatively replace the transcript outside a streaming turn.
   *  Surfaces use this to rehydrate the chat history from
   *  persisted turns on mount. The hook makes no attempt to
   *  reconcile a partial overlap with in-flight state — callers
   *  should only call this when `phase === "idle"`. */
  setTranscript: (entries: BuilderTranscriptEntry[]) => void;
  /** Count of `patch` events currently held in the R1 buffer. */
  bufferedPatches: number;
}

interface BufferedPatch {
  ev: Extract<BuilderEvent, { type: "patch" }>;
  expires: number;
}

/**
 * Headless ai-builder state machine. The view layer reads `tree` for
 * the canvas, `transcript` for the chat panel, and `phase` for the
 * composer status; transport is the adapter's concern.
 *
 * Patch-ordering invariant (per ai-builder SCOPE R1): if a `patch`
 * arrives whose `targetComponentId` is not in the current tree, it is
 * held for up to `patchBufferMs` and replayed once a parent lands.
 * After the window elapses, the buffered patch is dropped silently.
 */
export function useBuilder(opts: UseBuilderOptions): UseBuilderReturn {
  const {
    adapter,
    initialTree = null,
    defaultMode = "build",
    patchBufferMs = 2000,
    onTokenPatch,
    onShellPatch,
    onSessionArtifact,
    onSessionError,
    onError,
  } = opts;

  const [tree, setTree] = useState<UiComponentTree | null>(initialTree);
  const [transcript, setTranscript] = useState<BuilderTranscriptEntry[]>([]);
  const [phase, setPhase] = useState<BuilderPhase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<BuilderMode>(defaultMode);
  const [bufferedPatches, setBufferedPatches] = useState(0);
  const abortRef = useRef<AbortController | null>(null);
  const lastPromptRef = useRef<BuilderSendInput | null>(null);
  // Buffer lives outside React state — we replay it imperatively when
  // a render arrives. Re-rendering on every buffer mutation isn't
  // worth the noise.
  const bufferRef = useRef<BufferedPatch[]>([]);

  useEffect(() => () => abortRef.current?.abort(), []);

  const applyPatchIfPossible = useCallback(
    (current: UiComponentTree | null, ev: Extract<BuilderEvent, { type: "patch" }>) => {
      if (!current) return { tree: current, applied: false };
      if (!treeHasId(current.root, ev.targetComponentId)) {
        return { tree: current, applied: false };
      }
      return {
        tree: replaceAt(current, ev.targetComponentId, ev.subtree),
        applied: true,
      };
    },
    [],
  );

  const flushBuffer = useCallback(
    (latest: UiComponentTree | null): UiComponentTree | null => {
      if (!latest || bufferRef.current.length === 0) return latest;
      let next = latest;
      let didApply = true;
      // Re-run the buffer until a pass applies nothing — a single
      // patch landing can unblock siblings further down.
      while (didApply) {
        didApply = false;
        const kept: BufferedPatch[] = [];
        for (const buf of bufferRef.current) {
          const { tree: t, applied } = applyPatchIfPossible(next, buf.ev);
          next = t ?? next;
          if (applied) {
            didApply = true;
          } else {
            kept.push(buf);
          }
        }
        bufferRef.current = kept;
      }
      setBufferedPatches(bufferRef.current.length);
      return next;
    },
    [applyPatchIfPossible],
  );

  // Periodically expire stale buffered patches (R1 — drop after window).
  useEffect(() => {
    if (!patchBufferMs) return;
    const t = window.setInterval(() => {
      const now = Date.now();
      const before = bufferRef.current.length;
      bufferRef.current = bufferRef.current.filter((b) => b.expires > now);
      if (bufferRef.current.length !== before) {
        setBufferedPatches(bufferRef.current.length);
        if (
          typeof console !== "undefined" &&
          before - bufferRef.current.length > 0
        ) {
          console.warn(
            `[ai-builder] dropped ${
              before - bufferRef.current.length
            } stale Patch event(s) after ${patchBufferMs}ms`,
          );
        }
      }
    }, Math.max(250, Math.floor(patchBufferMs / 4)));
    return () => window.clearInterval(t);
  }, [patchBufferMs]);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setPhase("cancelled");
  }, []);

  const reset = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    bufferRef.current = [];
    setBufferedPatches(0);
    setTree(initialTree);
    setTranscript([]);
    setPhase("idle");
    setError(null);
    setMode(defaultMode);
    lastPromptRef.current = null;
  }, [defaultMode, initialTree]);

  const runTurn = useCallback(
    async (input: BuilderSendInput) => {
      const turnMode: BuilderMode = input.mode ?? "build";
      lastPromptRef.current = input;
      setTranscript((prev) => [
        ...prev,
        {
          id: makeId("u"),
          kind: "user",
          text: input.text,
          mode: turnMode,
          createdAt: Date.now(),
        },
      ]);
      setPhase("thinking");
      setError(null);

      const ctrl = new AbortController();
      abortRef.current = ctrl;

      try {
        for await (const ev of adapter.send(input, ctrl.signal)) {
          if (ctrl.signal.aborted) break;
          switch (ev.type) {
            case "full-render": {
              setTree(() => flushBuffer(ev.tree));
              setPhase((p) => (p === "thinking" ? "writing" : p));
              break;
            }
            case "patch": {
              setTree((curr) => {
                const { tree: next, applied } = applyPatchIfPossible(curr, ev);
                if (!applied) {
                  bufferRef.current.push({
                    ev,
                    expires: Date.now() + patchBufferMs,
                  });
                  setBufferedPatches(bufferRef.current.length);
                }
                return next ?? curr;
              });
              setPhase((p) => (p === "thinking" ? "writing" : p));
              break;
            }
            case "token-patch": {
              onTokenPatch?.(ev.patch);
              setPhase((p) => (p === "thinking" ? "writing" : p));
              break;
            }
            case "shell-patch": {
              onShellPatch?.(ev.patch);
              setPhase((p) => (p === "thinking" ? "writing" : p));
              break;
            }
            case "message": {
              // Ask-mode reply — surface the assistant's prose as a
              // transcript bubble. The canvas is intentionally not
              // touched here; build turns own the tree.
              setTranscript((prev) => [
                ...prev,
                {
                  id: makeId("a"),
                  kind: "assistant",
                  text: ev.text,
                  createdAt: Date.now(),
                },
              ]);
              setPhase((p) => (p === "thinking" ? "writing" : p));
              break;
            }
            case "session_artifact": {
              onSessionArtifact?.({
                sessionId: ev.session_id,
                key: ev.key,
                version: ev.version,
              });
              break;
            }
            case "session_error": {
              onSessionError?.(ev.error);
              break;
            }
            case "status": {
              setPhase(ev.phase);
              if (ev.message) {
                setTranscript((prev) => [
                  ...prev,
                  {
                    id: makeId("s"),
                    kind: "status",
                    text: ev.message!,
                    phase: ev.phase,
                    createdAt: Date.now(),
                  },
                ]);
              }
              if (ev.phase === "done" || ev.phase === "error") {
                // Server says we're done; stop reading even if the
                // adapter is generous with trailing frames.
                return;
              }
              break;
            }
            case "error": {
              throw new Error(ev.error);
            }
          }
        }
        setPhase("done");
      } catch (err) {
        if (ctrl.signal.aborted) {
          setPhase("cancelled");
          return;
        }
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setPhase("error");
        setTranscript((prev) => [
          ...prev,
          {
            id: makeId("e"),
            kind: "status",
            text: msg,
            phase: "error",
            createdAt: Date.now(),
          },
        ]);
        onError?.(err);
      } finally {
        if (abortRef.current === ctrl) abortRef.current = null;
      }
    },
    [
      adapter,
      applyPatchIfPossible,
      flushBuffer,
      onError,
      onSessionArtifact,
      onSessionError,
      onShellPatch,
      onTokenPatch,
      patchBufferMs,
    ],
  );

  const send = useCallback(
    async (raw: BuilderSendInput | string) => {
      const input: BuilderSendInput =
        typeof raw === "string" ? { text: raw, mode } : { mode, ...raw };
      if (!input.text.trim() && !input.slots) return;
      await runTurn(input);
    },
    [mode, runTurn],
  );

  const retry = useCallback(async () => {
    const last = lastPromptRef.current;
    if (!last) return;
    // Strip the trailing status frames from the prior turn so the
    // transcript doesn't accumulate retried noise.
    setTranscript((prev) => {
      let i = prev.length;
      while (i > 0 && prev[i - 1]?.kind === "status") i--;
      // Drop the most recent user entry too — runTurn re-appends it.
      if (i > 0 && prev[i - 1]?.kind === "user") i--;
      return prev.slice(0, i);
    });
    await runTurn(last);
  }, [runTurn]);

  return useMemo(
    () => ({
      tree,
      transcript,
      phase,
      error,
      mode,
      setMode,
      send,
      cancel,
      retry,
      reset,
      setTree,
      setTranscript,
      bufferedPatches,
    }),
    [
      tree,
      transcript,
      phase,
      error,
      mode,
      send,
      cancel,
      retry,
      reset,
      bufferedPatches,
    ],
  );
}
