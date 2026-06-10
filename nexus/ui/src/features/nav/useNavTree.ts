import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listNav } from "@/api/nav/list";
import type { NavNodeDetail } from "@/api/types";
import { buildNavTree, type NavTreeNode } from "@/features/nav/navTree";

export const navKey = ["nexus", "nav"] as const;

/** The caller's navigation tree (WS-13), already access-filtered server-side.
 *  Returns the nested forest for rendering plus the raw flat list (the builder
 *  edits the flat list). Loading/error are surfaced so the sidebar can show
 *  honest states without breaking the shell. */
export function useNavTree(): {
  tree: NavTreeNode[];
  nodes: NavNodeDetail[];
  isPending: boolean;
  isError: boolean;
} {
  const client = useStarterClient();
  const query = useQuery({
    queryKey: navKey,
    queryFn: () => listNav(client),
  });
  const nodes = query.data ?? [];
  return {
    tree: buildNavTree(nodes),
    nodes,
    isPending: query.isPending,
    isError: query.isError,
  };
}
