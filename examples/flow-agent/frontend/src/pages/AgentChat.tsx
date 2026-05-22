// Stage 4: live chat against /api/agents/{id}/run.
//
// `createSseAdapter` POSTs `{ input, history }` to the endpoint and
// parses `data: …` frames into `ChatStreamDelta`s. The backend already
// emits `{type:"text",text}` / `{type:"tool-call",toolCall}` /
// `{type:"error",error}` / `[DONE]` — which is exactly what the default
// adapter parser understands, so no custom `parse` callback is needed.
//
// Conversations persist per-agent via `persistence={{ key }}` (localStorage),
// and the composer accepts file attachments / paste / drag-drop.

import { useMemo } from "react";
import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Chat, createSseAdapter } from "@nube/starter-ui-chat";
import { useTranslate } from "@nube/starter-ui-core/i18n";

import { api } from "@/lib/api";

export function AgentChat() {
  const { id = "" } = useParams();
  const t = useTranslate();
  const agent = useQuery({
    queryKey: ["agent", id],
    queryFn: () => api.agents.get(id),
    enabled: Boolean(id),
  });

  const adapter = useMemo(
    () => createSseAdapter({ url: `/api/agents/${id}/run` }),
    [id],
  );

  const fallbackTitle = t("flow_agent.agent_chat.empty.fallback_title");

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col p-4">
      <Chat
        adapter={adapter}
        title={agent.data?.name ?? fallbackTitle}
        placeholder={t("flow_agent.agent_chat.placeholder")}
        emptyTitle={agent.data?.name ?? fallbackTitle}
        emptyDescription={
          agent.data
            ? `${agent.data.provider} · ${agent.data.model}`
            : t("flow_agent.agent_chat.empty.loading")
        }
        suggestions={[
          {
            label: t("flow_agent.agent_chat.suggestions.say_hi.label"),
            description: t("flow_agent.agent_chat.suggestions.say_hi.description"),
          },
          {
            label: t("flow_agent.agent_chat.suggestions.list_flows.label"),
            description: t("flow_agent.agent_chat.suggestions.list_flows.description"),
          },
          {
            label: t("flow_agent.agent_chat.suggestions.capabilities.label"),
            description: t("flow_agent.agent_chat.suggestions.capabilities.description"),
          },
        ]}
        persistence={{ key: `flow-agent:chat:${id}` }}
        allowAttachments
        showClearButton
        className="h-full"
      />
    </div>
  );
}
