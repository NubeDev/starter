import * as React from "react";
import { cn } from "../lib/utils.js";

export interface ChatRootProps extends React.HTMLAttributes<HTMLDivElement> {}

// Layout shell: column flex with header/messages/composer slots.
// Caller composes children freely.
export const ChatRoot = React.forwardRef<HTMLDivElement, ChatRootProps>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      data-slot="chat-root"
      className={cn(
        "flex h-full min-h-0 w-full flex-col bg-gradient-to-b from-background to-muted/30 text-foreground",
        className,
      )}
      {...props}
    />
  ),
);
ChatRoot.displayName = "ChatRoot";

export const ChatHeader = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    data-slot="chat-header"
    className={cn(
      "flex shrink-0 items-center gap-2 border-b border-border/60 bg-background/70 px-4 py-2.5 backdrop-blur",
      className,
    )}
    {...props}
  />
));
ChatHeader.displayName = "ChatHeader";

export const ChatFooter = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    data-slot="chat-footer"
    className={cn(
      "shrink-0 border-t border-border/60 bg-background/70 p-3 backdrop-blur",
      className,
    )}
    {...props}
  />
));
ChatFooter.displayName = "ChatFooter";
