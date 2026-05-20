import * as React from "react";
import { cn, formatTime } from "../lib/utils.js";
import type { ChatMessage as ChatMessageT, ChatRole } from "../types/index.js";

const roleAlign: Record<ChatRole, string> = {
  user: "items-end",
  assistant: "items-start",
  system: "items-center",
  tool: "items-start",
};

const roleBubble: Record<ChatRole, string> = {
  user: "bg-primary text-primary-foreground rounded-2xl rounded-br-sm",
  assistant: "bg-muted text-foreground rounded-2xl rounded-bl-sm",
  system: "bg-transparent text-muted-foreground text-xs italic",
  tool: "bg-secondary text-secondary-foreground rounded-xl border",
};

export interface ChatMessageProps extends React.HTMLAttributes<HTMLDivElement> {
  message: ChatMessageT;
  avatar?: React.ReactNode;
  name?: string;
  showTimestamp?: boolean;
  renderContent?: (m: ChatMessageT) => React.ReactNode;
}

export const ChatMessage = React.forwardRef<HTMLDivElement, ChatMessageProps>(
  (
    {
      message,
      avatar,
      name,
      showTimestamp = true,
      renderContent,
      className,
      ...props
    },
    ref,
  ) => {
    const isUser = message.role === "user";
    return (
      <div
        ref={ref}
        data-slot="chat-message"
        data-role={message.role}
        data-status={message.status}
        className={cn("flex w-full flex-col gap-1", roleAlign[message.role], className)}
        {...props}
      >
        <div
          className={cn(
            "flex max-w-[70%] gap-2",
            isUser ? "flex-row-reverse" : "flex-row",
          )}
        >
          {avatar ? (
            <div className="mt-0.5 shrink-0" aria-hidden>
              {avatar}
            </div>
          ) : null}
          <div className="flex flex-col gap-1">
            {name ? (
              <div className="text-xs font-medium text-muted-foreground">
                {name}
              </div>
            ) : null}
            <div
              className={cn(
                "whitespace-pre-wrap break-words px-3 py-2 text-sm leading-relaxed",
                roleBubble[message.role],
              )}
            >
              {renderContent
                ? renderContent(message)
                : message.content || (message.status === "streaming" ? "…" : "")}
            </div>
            {message.toolCalls?.length ? (
              <ChatToolCalls toolCalls={message.toolCalls} />
            ) : null}
            {showTimestamp ? (
              <div className="text-[10px] text-muted-foreground">
                {formatTime(message.createdAt)}
              </div>
            ) : null}
          </div>
        </div>
      </div>
    );
  },
);
ChatMessage.displayName = "ChatMessage";

function ChatToolCalls({
  toolCalls,
}: {
  toolCalls: NonNullable<ChatMessageT["toolCalls"]>;
}) {
  return (
    <div className="flex flex-col gap-1">
      {toolCalls.map((tc) => (
        <div
          key={tc.id}
          className="rounded-md border bg-secondary/40 px-2 py-1 text-xs"
          data-state={tc.state}
        >
          <span className="font-mono font-medium">{tc.name}</span>
          <span className="ml-2 text-muted-foreground">{tc.state}</span>
          {tc.error ? (
            <div className="mt-1 text-destructive">{tc.error}</div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
