import * as React from "react";
import { cn } from "../lib/utils.js";

export interface ChatEmptyProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  description?: string;
  suggestions?: Array<{ label: string; value?: string }>;
  onSuggestion?: (value: string) => void;
}

export const ChatEmpty = React.forwardRef<HTMLDivElement, ChatEmptyProps>(
  (
    {
      className,
      title = "How can I help?",
      description,
      suggestions,
      onSuggestion,
      children,
      ...props
    },
    ref,
  ) => (
    <div
      ref={ref}
      data-slot="chat-empty"
      className={cn(
        "m-auto flex max-w-md flex-col items-center justify-center gap-3 p-8 text-center",
        className,
      )}
      {...props}
    >
      <div className="text-lg font-semibold">{title}</div>
      {description ? (
        <div className="text-sm text-muted-foreground">{description}</div>
      ) : null}
      {children}
      {suggestions?.length ? (
        <div className="mt-2 flex flex-wrap justify-center gap-2">
          {suggestions.map((s) => (
            <button
              key={s.label}
              type="button"
              onClick={() => onSuggestion?.(s.value ?? s.label)}
              className="rounded-full border bg-background px-3 py-1.5 text-xs hover:bg-accent hover:text-accent-foreground"
            >
              {s.label}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  ),
);
ChatEmpty.displayName = "ChatEmpty";
