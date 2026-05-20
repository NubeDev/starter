import * as React from "react";
import { cn } from "../lib/utils.js";
import { useAutoScroll } from "../hooks/use-auto-scroll.js";

export interface ChatMessageListProps
  extends React.HTMLAttributes<HTMLDivElement> {
  // Pass the message-array (or its length) so auto-scroll re-pins on
  // every update.
  deps?: ReadonlyArray<unknown>;
}

export const ChatMessageList = React.forwardRef<
  HTMLDivElement,
  ChatMessageListProps
>(({ className, deps = [], children, ...props }, forwardedRef) => {
  const { ref } = useAutoScroll<HTMLDivElement>(deps);
  React.useImperativeHandle(
    forwardedRef,
    () => ref.current as HTMLDivElement,
    [ref],
  );
  return (
    <div
      ref={ref}
      data-slot="chat-message-list"
      className={cn(
        "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
});
ChatMessageList.displayName = "ChatMessageList";
