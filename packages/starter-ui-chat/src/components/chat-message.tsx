import * as React from "react";
import { cn, formatTime } from "../lib/utils.js";
import { formatBytes, isImageAttachment } from "../lib/attachments.js";
import { useChatMessages } from "../i18n/context.js";
import type {
  ChatAttachment,
  ChatMessage as ChatMessageT,
  ChatRole,
} from "../types/index.js";

const roleAlign: Record<ChatRole, string> = {
  user: "items-end",
  assistant: "items-start",
  system: "items-center",
  tool: "items-start",
};

const roleBubble: Record<ChatRole, string> = {
  user: "bg-primary text-primary-foreground rounded-2xl rounded-br-md shadow-sm",
  assistant:
    "bg-muted/60 text-foreground rounded-2xl rounded-bl-md border border-border/40",
  system:
    "bg-transparent text-muted-foreground text-xs italic max-w-[80%] text-center",
  tool: "bg-secondary/60 text-secondary-foreground rounded-xl border border-border/40",
};

export interface ChatMessageProps extends React.HTMLAttributes<HTMLDivElement> {
  message: ChatMessageT;
  avatar?: React.ReactNode;
  name?: string;
  showTimestamp?: boolean;
  /** Show copy/retry action buttons on hover. Default: true. */
  showActions?: boolean;
  /** Called when the user clicks the retry button. */
  onRetry?: (m: ChatMessageT) => void;
  renderContent?: (m: ChatMessageT) => React.ReactNode;
}

export const ChatMessage = React.forwardRef<HTMLDivElement, ChatMessageProps>(
  (
    {
      message,
      avatar,
      name,
      showTimestamp = true,
      showActions = true,
      onRetry,
      renderContent,
      className,
      ...props
    },
    ref,
  ) => {
    const isUser = message.role === "user";
    const isAssistant = message.role === "assistant";
    const isErrored = message.status === "error";
    const isStreaming = message.status === "streaming";
    const showRetry = !!onRetry && (isErrored || (isAssistant && !isStreaming));

    return (
      <div
        ref={ref}
        data-slot="chat-message"
        data-role={message.role}
        data-status={message.status}
        className={cn(
          "group/msg flex w-full flex-col gap-1",
          roleAlign[message.role],
          className,
        )}
        {...props}
      >
        <div
          className={cn(
            "flex max-w-[85%] gap-2 sm:max-w-[75%]",
            isUser ? "flex-row-reverse" : "flex-row",
          )}
        >
          {avatar !== undefined ? (
            <div className="mt-0.5 shrink-0" aria-hidden>
              {avatar}
            </div>
          ) : isAssistant ? (
            <DefaultAvatar />
          ) : null}
          <div className={cn("flex min-w-0 flex-col gap-1", isUser && "items-end")}>
            {name ? (
              <div className="px-1 text-xs font-medium text-muted-foreground">
                {name}
              </div>
            ) : null}
            {message.attachments?.length ? (
              <AttachmentGrid
                attachments={message.attachments}
                align={isUser ? "end" : "start"}
              />
            ) : null}
            {(message.content || isStreaming || !message.attachments?.length) && (
              <div
                className={cn(
                  "whitespace-pre-wrap break-words px-3.5 py-2 text-sm leading-relaxed",
                  roleBubble[message.role],
                  isErrored && "border-destructive/40 bg-destructive/10 text-destructive",
                )}
              >
                {renderContent ? (
                  renderContent(message)
                ) : message.content ? (
                  message.content
                ) : isStreaming ? (
                  <StreamingCursor />
                ) : isErrored ? (
                  "Something went wrong."
                ) : (
                  ""
                )}
              </div>
            )}
            {message.toolCalls?.length ? (
              <ChatToolCalls toolCalls={message.toolCalls} />
            ) : null}
            <div
              className={cn(
                "flex items-center gap-1 px-1 text-[10px] text-muted-foreground",
                isUser && "flex-row-reverse",
              )}
            >
              {showTimestamp ? <span>{formatTime(message.createdAt)}</span> : null}
              {showActions && message.content ? (
                <MessageActions
                  message={message}
                  showRetry={showRetry}
                  onRetry={onRetry}
                />
              ) : null}
            </div>
          </div>
        </div>
      </div>
    );
  },
);
ChatMessage.displayName = "ChatMessage";

function DefaultAvatar() {
  return (
    <div
      className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-primary/80 to-primary text-[10px] font-semibold text-primary-foreground"
      aria-hidden
    >
      AI
    </div>
  );
}

function StreamingCursor() {
  return (
    <span className="inline-flex items-center gap-1 text-muted-foreground">
      <span className="inline-block h-3 w-1.5 animate-pulse rounded-sm bg-current" />
    </span>
  );
}

function MessageActions({
  message,
  showRetry,
  onRetry,
}: {
  message: ChatMessageT;
  showRetry: boolean;
  onRetry?: (m: ChatMessageT) => void;
}) {
  const [copied, setCopied] = React.useState(false);
  const messages = useChatMessages();
  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(message.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore — secure context / permission missing
    }
  };
  return (
    <span className="inline-flex items-center gap-0.5 opacity-0 transition group-hover/msg:opacity-100 focus-within:opacity-100">
      <ActionButton
        label={copied ? messages.copied : messages.copy}
        onClick={onCopy}
      >
        {copied ? (
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="h-3 w-3"
            aria-hidden
          >
            <path d="M20 6L9 17l-5-5" />
          </svg>
        ) : (
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="h-3 w-3"
            aria-hidden
          >
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
          </svg>
        )}
      </ActionButton>
      {showRetry ? (
        <ActionButton label={messages.retry} onClick={() => onRetry?.(message)}>
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="h-3 w-3"
            aria-hidden
          >
            <path d="M21 12a9 9 0 11-3-6.7L21 8" />
            <path d="M21 3v5h-5" />
          </svg>
        </ActionButton>
      ) : null}
    </span>
  );
}

function ActionButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className="inline-flex h-5 w-5 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
    >
      {children}
    </button>
  );
}

function AttachmentGrid({
  attachments,
  align,
}: {
  attachments: ChatAttachment[];
  align: "start" | "end";
}) {
  return (
    <div
      className={cn(
        "flex flex-wrap gap-2",
        align === "end" ? "justify-end" : "justify-start",
      )}
    >
      {attachments.map((a) => (
        <AttachmentChip key={a.id} attachment={a} />
      ))}
    </div>
  );
}

function AttachmentChip({ attachment }: { attachment: ChatAttachment }) {
  if (isImageAttachment(attachment) && attachment.url) {
    return (
      <a
        href={attachment.url}
        target="_blank"
        rel="noreferrer noopener"
        className="block overflow-hidden rounded-xl border border-border/40 bg-muted/40 transition hover:opacity-90"
        title={attachment.name}
      >
        <img
          src={attachment.url}
          alt={attachment.name}
          className="max-h-48 max-w-xs object-cover"
        />
      </a>
    );
  }
  const inner = (
    <div className="flex items-center gap-2 rounded-xl border border-border/40 bg-muted/40 px-2.5 py-1.5 text-xs">
      <span className="flex h-8 w-8 items-center justify-center rounded-md bg-background text-[10px] font-semibold uppercase text-muted-foreground">
        {(attachment.name.split(".").pop() || "file").slice(0, 4)}
      </span>
      <span className="flex flex-col">
        <span className="max-w-[10rem] truncate font-medium">
          {attachment.name}
        </span>
        {attachment.sizeBytes ? (
          <span className="text-[10px] text-muted-foreground">
            {formatBytes(attachment.sizeBytes)}
          </span>
        ) : null}
      </span>
    </div>
  );
  return attachment.url ? (
    <a
      href={attachment.url}
      target="_blank"
      rel="noreferrer noopener"
      className="hover:opacity-90"
    >
      {inner}
    </a>
  ) : (
    inner
  );
}

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
          className="rounded-lg border border-border/40 bg-secondary/30 px-2.5 py-1.5 text-xs"
          data-state={tc.state}
        >
          <span className="font-mono font-medium">{tc.name}</span>
          <span
            className={cn(
              "ml-2 rounded-full px-1.5 py-0.5 text-[10px] uppercase tracking-wide",
              tc.state === "running" &&
                "bg-primary/15 text-primary animate-pulse",
              tc.state === "done" && "bg-emerald-500/15 text-emerald-600",
              tc.state === "error" && "bg-destructive/15 text-destructive",
              tc.state === "pending" && "bg-muted text-muted-foreground",
            )}
          >
            {tc.state}
          </span>
          {tc.error ? (
            <div className="mt-1 text-destructive">{tc.error}</div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
