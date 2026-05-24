import * as React from "react";
import { cn } from "../lib/utils.js";
import { useChatMessages } from "../i18n/context.js";

export interface ChatEmptySuggestion {
  label: string;
  value?: string;
  description?: string;
  icon?: React.ReactNode;
}

export interface ChatEmptyProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  description?: string;
  icon?: React.ReactNode;
  suggestions?: Array<ChatEmptySuggestion>;
  onSuggestion?: (value: string) => void;
}

export const ChatEmpty = React.forwardRef<HTMLDivElement, ChatEmptyProps>(
  (
    {
      className,
      title,
      description,
      icon,
      suggestions,
      onSuggestion,
      children,
      ...props
    },
    ref,
  ) => {
    const messages = useChatMessages();
    const resolvedTitle = title ?? messages.emptyTitle;
    return (
    <div
      ref={ref}
      data-slot="chat-empty"
      className={cn(
        "m-auto flex w-full max-w-xl flex-col items-center justify-center gap-4 p-6 text-center",
        className,
      )}
      {...props}
    >
      <div
        className="flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-primary/15 to-primary/5 text-primary"
        aria-hidden
      >
        {icon ?? (
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="h-6 w-6"
            aria-hidden
          >
            <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
          </svg>
        )}
      </div>
      <div className="flex flex-col gap-1">
        <div className="text-lg font-semibold tracking-tight">{resolvedTitle}</div>
        {description ? (
          <div className="text-sm text-muted-foreground">{description}</div>
        ) : null}
      </div>
      {children}
      {suggestions?.length ? (
        <div className="mt-2 grid w-full gap-2 sm:grid-cols-2">
          {suggestions.map((s) => (
            <button
              key={s.label}
              type="button"
              onClick={() => onSuggestion?.(s.value ?? s.label)}
              className="group flex flex-col gap-1 rounded-xl border border-border/60 bg-background/60 p-3 text-left text-sm shadow-sm transition hover:-translate-y-0.5 hover:border-primary/40 hover:bg-background hover:shadow-md"
            >
              <span className="flex items-center gap-2 font-medium">
                {s.icon ? (
                  <span className="text-muted-foreground group-hover:text-primary">
                    {s.icon}
                  </span>
                ) : null}
                <span>{s.label}</span>
              </span>
              {s.description ? (
                <span className="text-xs text-muted-foreground">
                  {s.description}
                </span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
    );
  },
);
ChatEmpty.displayName = "ChatEmpty";
