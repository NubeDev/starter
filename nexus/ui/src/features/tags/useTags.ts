import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getTags } from "@/api/tags/get";
import { listTagKeys } from "@/api/tags/keys";
import { setTags } from "@/api/tags/set";
import type { SetTagsRequest, Tag, TaggableKind } from "@/api/types";

// One query key per entity's tag set; a mutation on that entity invalidates
// exactly its own tags.
const tagsKey = (kind: TaggableKind, id: string) =>
  ["nexus", "tags", kind, id] as const;
const KEYS_KEY = ["nexus", "tags", "keys"] as const;

// The tags on one entity. Returns the full query result so the editor can
// render loading/error states.
export function useTags(
  kind: TaggableKind,
  id: string,
): UseQueryResult<Tag[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: tagsKey(kind, id),
    queryFn: () => getTags(client, kind, id),
    staleTime: 60_000,
  });
}

// The distinct tag keys across the tenant, for key autocomplete.
export function useTagKeys(): UseQueryResult<string[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEYS_KEY,
    queryFn: () => listTagKeys(client),
    staleTime: 60_000,
  });
}

// Replace an entity's full tag set. On success, refresh that entity's tags
// and the tenant-wide key list (a new key may have appeared).
export function useSetTags(kind: TaggableKind, id: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, SetTagsRequest>({
    mutationFn: (body) => setTags(client, kind, id, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: tagsKey(kind, id) });
      void queryClient.invalidateQueries({ queryKey: KEYS_KEY });
    },
  });
}
