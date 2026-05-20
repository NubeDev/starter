// Stage 4: live chat against /api/agents/{id}/run.
//
// `createSseAdapter` POSTs `{ input, history }` to the endpoint and
// parses `data: …` frames into `ChatStreamDelta`s. The backend already
// emits `{type:"text",text}` / `{type:"tool-call",toolCall}` /
// `{type:"error",error}` / `[DONE]` — which is exactly what the default
// adapter parser understands, so no custom `parse` callback is needed.
//
// Conversations are not persisted; we wrap the adapter in `useMemo`
// keyed on `id` so navigating between agents starts a fresh history.

import { useMemo } from "react";
import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Chat, createSseAdapter } from "@nube/starter-ui-chat";

import { api } from "../lib/api";

export function AgentChat() {
  const { id = "" } = useParams();
  const agent = useQuery({
    queryKey: ["agent", id],
    queryFn: () => api.agents.get(id),
    enabled: Boolean(id),
  });

  const adapter = useMemo(
    () => createSseAdapter({ url: `/api/agents/${id}/run` }),
    [id],
  );

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col p-4">
      <Chat
        adapter={adapter}
        title={agent.data?.name ?? "Agent"}
        placeholder="Message the agent…"
        emptyTitle={agent.data?.name ?? "Agent"}
        emptyDescription={
          agent.data
            ? `${agent.data.provider} · ${agent.data.model}`
            : "Loading agent…"
        }
        suggestions={[
          { label: "Say hi" },
          { label: "List my flows" },
          { label: "What can you do?" },
        ]}
        className="h-full"
      />
    </div>
  );
}
