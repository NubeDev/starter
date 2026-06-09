// Chatbot session hook: drives one agent conversation with live SSE streaming.
//
// Flow (mirrors useLiveStream's F5 contract): POST /agents/{id}/sessions returns
// { id, token }; we open the session's SSE feed at .../events?token=… and fold
// each event into the streaming assistant turn. TextDelta appends; Done finalises
// (and carries the full text when the backend didn't stream deltas, e.g. the zag
// agent tier). A Raw {error} event surfaces as an error on the turn.
//
// The conversation is kept in local state (the backend persists each session's
// transcript independently). Each send starts a fresh session — the v1 agent
// session is single-turn — but the UI accumulates turns so it reads as a chat.
import { useCallback, useRef, useState } from "react";
import { streamJson } from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";

import {
  agentSessionEventsUrl,
  createAgentSession,
  getAgentSession,
} from "@/api/agents";

/** A unified agent event as emitted by the backend SSE feed. Mirrors the
 * nexus-ai `Event` enum: a tagged union on `kind`. */
type AgentEvent =
  | { kind: "text_delta"; text: string }
  | { kind: "tool_call"; name: string; input: unknown }
  | { kind: "progress"; message: string }
  | { kind: "done"; text: string }
  | { kind: "raw"; [k: string]: unknown };

export type ChatRole = "user" | "assistant";

export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  /** True while this assistant turn is still streaming. */
  streaming?: boolean;
  /** Set when the run failed. */
  error?: string;
}

export interface UseChat {
  messages: ChatMessage[];
  /** A run is in flight (awaiting/streaming the assistant reply). */
  busy: boolean;
  /** Send a user message to the agent and stream the reply. */
  send: (text: string) => Promise<void>;
  /** Clear the conversation. */
  reset: () => void;
}

let seq = 0;
const nextId = () => `m${Date.now()}-${seq++}`;

/** Drive a chat against `agentId`. Returns the running transcript plus `send`. */
export function useChat(agentId: string | undefined): UseChat {
  const client = useStarterClient();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [busy, setBusy] = useState(false);
  const abort = useRef<AbortController | null>(null);

  const reset = useCallback(() => {
    abort.current?.abort();
    abort.current = null;
    setMessages([]);
    setBusy(false);
  }, []);

  const send = useCallback(
    async (text: string) => {
      const prompt = text.trim();
      if (!agentId || !prompt || busy) return;

      const userMsg: ChatMessage = { id: nextId(), role: "user", content: prompt };
      const replyId = nextId();
      setMessages((m) => [
        ...m,
        userMsg,
        { id: replyId, role: "assistant", content: "", streaming: true },
      ]);
      setBusy(true);

      const ctrl = new AbortController();
      abort.current = ctrl;

      const patchReply = (fn: (msg: ChatMessage) => ChatMessage) =>
        setMessages((m) => m.map((msg) => (msg.id === replyId ? fn(msg) : msg)));

      let sessionId: string | undefined;
      try {
        const session = await createAgentSession(client, agentId, { prompt });
        sessionId = session.id;
        const url = agentSessionEventsUrl(client, session.id, session.token);
        // A terminal event (`done` or an error `raw`) ends the turn. We must stop
        // iterating AND abort, because a browser EventSource auto-reconnects on
        // any stream close — without aborting it would re-open the feed in a loop.
        let terminal = false;
        for await (const ev of streamJson<AgentEvent>(client, url, {
          signal: ctrl.signal,
        })) {
          switch (ev.kind) {
            case "text_delta":
              patchReply((msg) => ({ ...msg, content: msg.content + ev.text }));
              break;
            case "done":
              patchReply((msg) => ({
                ...msg,
                content: ev.text.length > 0 ? ev.text : msg.content,
                streaming: false,
              }));
              terminal = true;
              break;
            case "raw":
              if (typeof ev.error === "string") {
                patchReply((msg) => ({ ...msg, error: ev.error as string, streaming: false }));
                terminal = true;
              }
              break;
            // tool_call / progress: not surfaced in the v1 chat UI.
            default:
              break;
          }
          if (terminal) {
            ctrl.abort(); // close the EventSource so it doesn't reconnect
            break;
          }
        }
        // The stream ended without a terminal event (the run finished faster than
        // we attached, or closed empty). The transcript is the durable source of
        // truth — fetch it so the reply is never silently empty.
        if (!terminal) {
          await settleFromSession(client, sessionId, patchReply);
        }
      } catch (err) {
        if (ctrl.signal.aborted) return;
        // A transport error mid-stream: fall back to the persisted session before
        // surfacing a raw error, so a completed run still shows its answer.
        const settled = await settleFromSession(client, sessionId, patchReply);
        if (!settled) {
          patchReply((msg) => ({
            ...msg,
            streaming: false,
            error: err instanceof Error ? err.message : "The agent run failed.",
          }));
        }
      } finally {
        if (abort.current === ctrl) abort.current = null;
        setBusy(false);
      }
    },
    [agentId, busy, client],
  );

  return { messages, busy, send, reset };
}

/** Settle the streaming reply from the persisted session (the durable source of
 * truth) when the SSE stream produced no terminal event. Returns true if it
 * applied a final state (answer or error). Best-effort: a fetch failure returns
 * false so the caller can fall back to surfacing the stream error. */
async function settleFromSession(
  client: ReturnType<typeof useStarterClient>,
  sessionId: string | undefined,
  patchReply: (fn: (msg: ChatMessage) => ChatMessage) => void,
): Promise<boolean> {
  if (!sessionId) return false;
  try {
    const session = await getAgentSession(client, sessionId);
    if (session.status === "failed" || session.status === "cancelled") {
      patchReply((msg) => ({
        ...msg,
        streaming: false,
        error: `The agent run ${session.status}.`,
      }));
      return true;
    }
    const text = assistantText(session.transcript);
    patchReply((msg) => ({
      ...msg,
      content: text && text.length > 0 ? text : msg.content,
      streaming: false,
    }));
    return true;
  } catch {
    return false;
  }
}

/** Pull the assistant turn's content from a persisted `[{role,content},…]`
 * transcript (the SessionDetail.transcript is opaque JSON). */
function assistantText(transcript: unknown): string | undefined {
  if (!Array.isArray(transcript)) return undefined;
  for (let i = transcript.length - 1; i >= 0; i--) {
    const m = transcript[i] as { role?: unknown; content?: unknown };
    if (m && m.role === "assistant" && typeof m.content === "string") {
      return m.content;
    }
  }
  return undefined;
}
