import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  createDetection,
  detectionStats,
  listDetections,
  removeDetection,
  runDetection,
  updateDetection,
} from "@/api/detections/detections";
import {
  ackFinding,
  listFindings,
  resolveFinding,
  type FindingFilter,
} from "@/api/detections/findings";
import { listInsights } from "@/api/insights/list";
import type {
  CreateDetectionRequest,
  DetectionDetail,
  DetectionStats,
  Finding,
  InsightSummary,
  UpdateDetectionRequest,
} from "@/api/types";

const KEY = {
  detections: ["nexus", "detections"] as const,
  insights: ["nexus", "insights"] as const,
  stats: (id: string) => ["nexus", "detections", id, "stats"] as const,
  findings: (filter: FindingFilter) =>
    ["nexus", "findings", filter.detectionId ?? null, filter.status ?? null] as const,
};

// Detections list + the insights catalog the detection editor picks from.
export function useDetections(): UseQueryResult<DetectionDetail[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEY.detections,
    queryFn: () => listDetections(client),
  });
}

export function useInsightOptions(): UseQueryResult<InsightSummary[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.insights, queryFn: () => listInsights(client) });
}

// Run stats for one detection (next run + findings-by-status). Polls on the
// findings cadence so the row reflects a recent run without a manual refresh.
export function useDetectionStats(id: string): UseQueryResult<DetectionStats> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEY.stats(id),
    queryFn: () => detectionStats(client, id),
    staleTime: 15_000,
  });
}

// Findings under a filter. Kept briefly fresh: the runner writes on its own
// cadence, so a short stale window picks up new sparks without hammering.
export function useFindings(
  filter: FindingFilter = {},
): UseQueryResult<Finding[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEY.findings(filter),
    queryFn: () => listFindings(client, filter),
    staleTime: 15_000,
  });
}

export function useDetectionMutations() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidateDetections = () =>
    queryClient.invalidateQueries({ queryKey: KEY.detections });
  const invalidateFindings = () =>
    queryClient.invalidateQueries({ queryKey: ["nexus", "findings"] });
  const invalidateStats = () =>
    queryClient.invalidateQueries({ queryKey: ["nexus", "detections"] });
  return {
    create: useMutation<DetectionDetail, Error, CreateDetectionRequest>({
      mutationFn: (body) => createDetection(client, body),
      onSuccess: invalidateDetections,
    }),
    update: useMutation<void, Error, { id: string; patch: UpdateDetectionRequest }>({
      mutationFn: ({ id, patch }) => updateDetection(client, id, patch),
      onSuccess: invalidateDetections,
    }),
    remove: useMutation<void, Error, string>({
      mutationFn: (id) => removeDetection(client, id),
      onSuccess: () => {
        invalidateDetections();
        // A detection's findings cascade on delete — refresh the feed too.
        invalidateFindings();
      },
    }),
    // Run now, then refresh findings + stats so the result shows without a wait.
    run: useMutation<void, Error, string>({
      mutationFn: (id) => runDetection(client, id),
      onSuccess: () => {
        invalidateFindings();
        invalidateStats();
      },
    }),
  };
}

export function useFindingMutations() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["nexus", "findings"] });
  return {
    ack: useMutation<void, Error, { id: string; note?: string }>({
      mutationFn: ({ id, note }) => ackFinding(client, id, { note: note ?? null }),
      onSuccess: invalidate,
    }),
    resolve: useMutation<void, Error, { id: string; note?: string }>({
      mutationFn: ({ id, note }) =>
        resolveFinding(client, id, { note: note ?? null }),
      onSuccess: invalidate,
    }),
  };
}
