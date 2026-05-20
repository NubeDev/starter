// Phase 1 stub. Phase 2 wires <FlowCanvas> here.

import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";

import { api } from "../lib/api";

export function FlowEditor() {
  const { id = "" } = useParams();
  const flow = useQuery({
    queryKey: ["flow", id],
    queryFn: () => api.flows.get(id),
    enabled: !!id,
  });

  return (
    <div className="flex h-full flex-col">
      <header className="border-b border-border/60 px-6 py-3">
        <h1 className="text-lg font-semibold tracking-tight">
          {flow.data?.name ?? "…"}
        </h1>
        <p className="text-xs text-muted-foreground">{id}</p>
      </header>
      <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
        Flow canvas lands in Phase 2.
      </div>
    </div>
  );
}
