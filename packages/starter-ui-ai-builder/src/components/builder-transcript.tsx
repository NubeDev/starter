import * as React from "react";
import {
  ChatComposer,
  ChatFooter,
  ChatMessageList,
  ChatRoot,
  type ChatStatus,
} from "@nube/starter-ui-chat";
import { cn } from "../lib/utils.js";
import type { BuilderPhase } from "../types/index.js";
import type { BuilderTranscriptEntry } from "../hooks/use-builder.js";

export interface BuilderTranscriptProps {
  entries: BuilderTranscriptEntry[];
  phase: BuilderPhase;
  /** Streaming label shown while `phase` is "thinking" or "writing". */
  busyLabel?: string;
  /** Allow attachments on the composer. */
  allowAttachments?: boolean;
  placeholder?: string;
  onSend: (text: string) => void;
  onCancel?: () => void;
  onRetry?: () => void;
  /** Show a Retry button below the last assistant turn when phase is
   *  `done` / `error` / `cancelled` and there is at least one prior
   *  user prompt. */
  canRetry?: boolean;
  className?: string;
}

// A chat-style transcript pane that pairs naturally with
// `<AiBuilderCanvas>`. Reuses starter-ui-chat primitives for the
// composer and the scrollable surface; renders our own entry shapes
// (user prompts + status frames) — the canvas IS the AI's "output",
// so we don't fake assistant bubbles.
export function BuilderTranscript(
  props: BuilderTranscriptProps,
): React.ReactElement {
  const {
    entries,
    phase,
    busyLabel = "Working…",
    allowAttachments,
    placeholder = "Describe the UI you want…",
    onSend,
    onCancel,
    onRetry,
    canRetry,
    className,
  } = props;

  const composerStatus: ChatStatus =
    phase === "thinking"
      ? "submitted"
      : phase === "writing"
        ? "streaming"
        : phase === "error"
          ? "error"
          : phase === "cancelled"
            ? "cancelled"
            : phase === "done"
              ? "done"
              : "idle";

  const showBusy = phase === "thinking" || phase === "writing";
  const showRetry = !!canRetry && !!onRetry && !showBusy && entries.length > 0;

  return (
    <ChatRoot className={cn("h-full", className)}>
      <ChatMessageList deps={[entries.length, entries[entries.length - 1]?.text, phase]}>
        {entries.length === 0 ? (
          <div className="m-auto max-w-sm p-6 text-center text-sm text-muted-foreground">
            Tell the agent what to build. Updates stream into the
            canvas on the right.
          </div>
        ) : (
          entries.map((e) => <TranscriptItem key={e.id} entry={e} />)
        )}
        {showBusy ? <BusyBubble label={busyLabel} phase={phase} /> : null}
        {showRetry ? (
          <div className="flex">
            <button
              type="button"
              onClick={onRetry}
              className="inline-flex items-center gap-1.5 rounded-md border border-border/60 bg-background px-2.5 py-1 text-xs text-muted-foreground transition hover:bg-muted hover:text-foreground"
            >
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
              Regenerate
            </button>
          </div>
        ) : null}
      </ChatMessageList>
      <ChatFooter>
        <ChatComposer
          status={composerStatus}
          placeholder={placeholder}
          allowAttachments={allowAttachments}
          onSend={(input) => onSend(input.text)}
          onCancel={onCancel}
        />
      </ChatFooter>
    </ChatRoot>
  );
}

function TranscriptItem({ entry }: { entry: BuilderTranscriptEntry }) {
  if (entry.kind === "user") {
    return (
      <div className="flex w-full flex-col items-end gap-1">
        <div className="max-w-[85%] whitespace-pre-wrap break-words rounded-2xl rounded-br-md bg-primary px-3.5 py-2 text-sm text-primary-foreground shadow-sm">
          {entry.text}
        </div>
      </div>
    );
  }
  // status frame
  const tone =
    entry.phase === "error"
      ? "border-destructive/40 bg-destructive/10 text-destructive"
      : entry.phase === "done"
        ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
        : "border-border/40 bg-muted/40 text-muted-foreground";
  return (
    <div className={cn("flex w-full")}>
      <div
        className={cn(
          "inline-flex max-w-[85%] items-start gap-2 rounded-xl border px-3 py-1.5 text-xs",
          tone,
        )}
      >
        <span
          aria-hidden
          className={cn(
            "mt-0.5 inline-block h-1.5 w-1.5 rounded-full",
            entry.phase === "error"
              ? "bg-destructive"
              : entry.phase === "done"
                ? "bg-emerald-500"
                : "bg-muted-foreground/60",
          )}
        />
        <span className="whitespace-pre-wrap break-words">{entry.text}</span>
      </div>
    </div>
  );
}

function BusyBubble({ label, phase }: { label: string; phase: BuilderPhase }) {
  return (
    <div className="flex">
      <div className="inline-flex items-center gap-2 rounded-2xl rounded-bl-md border border-border/40 bg-muted/60 px-3.5 py-2 text-xs text-muted-foreground">
        <span className="inline-flex items-center gap-1">
          <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-current [animation-delay:-0.3s]" />
          <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-current [animation-delay:-0.15s]" />
          <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-current" />
        </span>
        <span>{label}</span>
        <span className="text-[10px] uppercase tracking-wide opacity-60">
          {phase}
        </span>
      </div>
    </div>
  );
}
