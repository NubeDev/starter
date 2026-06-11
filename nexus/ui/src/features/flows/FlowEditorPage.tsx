import { useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { ArrowLeft, Bug } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { FlowEditor } from "@/features/flows/builder/FlowBuilder";
import { useFlows } from "@/features/flows/useFlows";
import { useFlowDebug } from "@/features/flows/useFlowDebug";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Full-page flow editor at `/flows/:flowName`. Edit and Debug are the SAME view:
// the canvas, the right-hand node-config panel, and (when debugging) a live dock
// below it all share one selection. Turning Debug on just overlays live counters
// on the canvas and reveals the dock — it never swaps you to a separate screen,
// so node settings stay visible the whole time.
export function FlowEditorPage() {
  const { flowName = "" } = useParams();
  const navigate = useNavigate();
  const decoded = decodeURIComponent(flowName);
  const [params, setParams] = useSearchParams();
  const { data, isPending, isError, error } = useFlows();
  // `?debug=1` (set by the list's Debug button, or by sharing the link) starts
  // in debug mode.
  const [debugging, setDebugging] = useState(params.get("debug") === "1");

  const flow = data?.find((f) => f.name === decoded);
  // Subscribe to the live stream only while debugging a running flow. Enabling
  // capture and the SSE connection are driven by the `active` flag here.
  const debugActive = debugging && !!flow?.running;
  const debug = useFlowDebug(flow?.id ?? null, debugActive);

  if (isPending) return <Loading label="Loading flow…" />;
  if (isError) {
    return (
      <ErrorState message={error instanceof Error ? error.message : undefined} />
    );
  }
  if (!flow) {
    return (
      <Empty
        title="Flow not found"
        description={`No flow named "${decoded}". It may have been renamed or deleted.`}
      />
    );
  }

  // Keep `?debug` in the URL in sync so the page is shareable and survives a
  // refresh.
  const setDebug = (on: boolean) => {
    setDebugging(on);
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (on) next.set("debug", "1");
        else next.delete("debug");
        return next;
      },
      { replace: true },
    );
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Back to flows"
            onClick={() => navigate("/flows")}
            className="text-muted-foreground hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
          </Button>
          <h2 className="truncate text-base font-semibold tracking-tight">
            {flow.name}
          </h2>
          <span
            className="size-1.5 rounded-full"
            style={{
              backgroundColor: flow.running
                ? "var(--chart-1)"
                : "var(--muted-foreground)",
              boxShadow: flow.running ? "0 0 8px var(--chart-1)" : undefined,
            }}
            aria-hidden
          />
        </div>
        {/* One toggle flips the same view between plain edit and edit + live
            overlay. Only meaningful while the flow is running. */}
        <Button
          variant={debugging ? "default" : "outline"}
          size="sm"
          className="gap-2"
          disabled={!flow.running}
          title={flow.running ? "Overlay live values on the canvas" : "Start the flow to debug it"}
          onClick={() => setDebug(!debugging)}
        >
          <Bug className="size-4" />
          {debugging ? "Debugging" : "Debug"}
        </Button>
      </div>

      <div className="min-h-0 flex-1">
        <FlowEditor
          key={flow.id}
          flowId={flow.id}
          onDone={() => navigate("/flows")}
          debug={debugActive ? debug : undefined}
        />
      </div>
    </div>
  );
}
