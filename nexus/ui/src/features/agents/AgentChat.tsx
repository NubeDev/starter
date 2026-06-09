import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Plus, Send } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";
import { cn } from "@nube/starter-ui-kit/lib/utils";

import type { AgentDetail, AgentSummary } from "@/api/types";
import { useChat, type ChatMessage } from "@/features/agents/useChat";

// The chatbot panel for a selected agent — the test surface that proves an
// agent works. Streams the reply turn-by-turn via `useChat`, with a composer
// that sends on Enter (Shift+Enter for a newline) and a "New chat" reset.
export function AgentChat({ agent }: { agent: AgentSummary | AgentDetail }) {
  const { messages, busy, send, reset } = useChat(agent.id);
  const [draft, setDraft] = useState("");
  const endRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to the latest turn whenever the transcript grows or the
  // streaming content changes.
  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages]);

  function submit() {
    const text = draft.trim();
    if (!text || busy) return;
    setDraft("");
    void send(text);
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-foreground">
            {agent.name}
          </p>
          <p className="text-xs text-muted-foreground">
            {agent.backend} · {agent.model}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={reset}
          disabled={messages.length === 0 && !busy}
        >
          <Plus className="size-4" />
          New chat
        </Button>
      </div>

      <div className="scrollbar-thin min-h-0 flex-1 overflow-y-auto">
        {messages.length === 0 ? (
          <div className="flex h-full min-h-48 flex-col items-center justify-center gap-2 text-center">
            <p className="text-sm font-medium text-foreground">
              Start a conversation
            </p>
            <p className="max-w-sm text-sm text-muted-foreground">
              Say hi to {agent.name} to check the agent works.
            </p>
          </div>
        ) : (
          <ul className="flex flex-col gap-3">
            {messages.map((msg) => (
              <Bubble key={msg.id} message={msg} />
            ))}
            <div ref={endRef} />
          </ul>
        )}
      </div>

      <div className="flex items-end gap-2">
        <Textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Message the agent…  (Enter to send, Shift+Enter for a newline)"
          spellCheck={false}
          className="min-h-11 resize-y text-sm"
        />
        <Button
          type="button"
          size="icon"
          aria-label="Send message"
          onClick={submit}
          disabled={busy || draft.trim() === ""}
        >
          <Send className="size-4" />
        </Button>
      </div>
    </div>
  );
}

function Bubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  return (
    <li className={cn("flex", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-[85%] whitespace-pre-wrap rounded-lg px-3 py-2 text-sm",
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-muted text-foreground",
        )}
      >
        {message.content}
        {message.streaming ? (
          <span className="ms-0.5 inline-block animate-pulse">▍</span>
        ) : null}
        {message.error ? (
          <p role="alert" className="mt-1 text-sm text-destructive">
            {message.error}
          </p>
        ) : null}
      </div>
    </li>
  );
}
