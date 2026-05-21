import * as React from "react";
import { cn } from "../lib/utils.js";

export interface ChatTypingIndicatorProps
  extends React.HTMLAttributes<HTMLDivElement> {
  label?: string;
}

export const ChatTypingIndicator = React.forwardRef<
  HTMLDivElement,
  ChatTypingIndicatorProps
>(({ className, label = "Assistant is typing", ...props }, ref) => (
  <div
    ref={ref}
    role="status"
    aria-label={label}
    data-slot="chat-typing-indicator"
    className={cn(
      "inline-flex items-center gap-1.5 rounded-2xl rounded-bl-md border border-border/40 bg-muted/60 px-3.5 py-2.5",
      className,
    )}
    {...props}
  >
    <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/70 [animation-delay:-0.3s]" />
    <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/70 [animation-delay:-0.15s]" />
    <span className="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/70" />
  </div>
));
ChatTypingIndicator.displayName = "ChatTypingIndicator";
