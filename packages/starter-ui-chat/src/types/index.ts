// Core domain types for the chat library. Transport-agnostic.

export type ChatRole = "user" | "assistant" | "system" | "tool";

export type ChatStatus =
  | "idle"
  | "submitted"
  | "streaming"
  | "done"
  | "error"
  | "cancelled";

export interface ChatAttachment {
  id: string;
  name: string;
  mimeType: string;
  url?: string;
  sizeBytes?: number;
}

export interface ChatToolCall {
  id: string;
  name: string;
  args: unknown;
  result?: unknown;
  state: "pending" | "running" | "done" | "error";
  error?: string;
}

export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  createdAt: number;
  status?: ChatStatus;
  attachments?: ChatAttachment[];
  toolCalls?: ChatToolCall[];
  meta?: Record<string, unknown>;
}

export interface ChatStreamDelta {
  type: "text" | "tool-call" | "tool-result" | "status" | "error" | "done";
  text?: string;
  toolCall?: ChatToolCall;
  status?: ChatStatus;
  error?: string;
}

export interface ChatSendInput {
  text: string;
  attachments?: ChatAttachment[];
  meta?: Record<string, unknown>;
}

// The single transport seam. The library never speaks HTTP/SSE itself —
// the consumer wires whatever stack they use (fetch, EventSource, MCP,
// starter-ai, mock for tests) behind this interface.
export interface ChatAdapter {
  send(
    input: ChatSendInput,
    history: ReadonlyArray<ChatMessage>,
    signal: AbortSignal,
  ): AsyncIterable<ChatStreamDelta>;
}
