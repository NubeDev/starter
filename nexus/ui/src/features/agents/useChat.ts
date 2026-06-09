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

import { agentSessionEventsUrl, createAgentSession } from "@/api/agents";

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

      try {
        const session = await createAgentSession(client, agentId, { prompt });
        const url = agentSessionEventsUrl(client, session.id, session.token);
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
              break;
            case "raw":
              if (typeof ev.error === "string") {
                patchReply((msg) => ({ ...msg, error: ev.error as string, streaming: false }));
              }
              break;
            // tool_call / progress: not surfaced in the v1 chat UI.
            default:
              break;
          }
        }
        // Stream closed without an explicit done (e.g. completed): mark settled.
        patchReply((msg) => (msg.streaming ? { ...msg, streaming: false } : msg));
      } catch (err) {
        if (ctrl.signal.aborted) return;
        patchReply((msg) => ({
          ...msg,
          streaming: false,
          error: err instanceof Error ? err.message : "The agent run failed.",
        }));
      } finally {
        if (abort.current === ctrl) abort.current = null;
        setBusy(false);
      }
    },
    [agentId, busy, client],
  );

  return { messages, busy, send, reset };
}
