// Phase 1 stub. Phase 4 wires <Chat /> from @nube/starter-ui-chat
// over the /api/agents/:id/run SSE endpoint.

import { useParams } from "react-router-dom";

export function AgentChat() {
  const { id } = useParams();
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      Chat for agent <code className="ml-1">{id}</code> — Phase 4.
    </div>
  );
}
