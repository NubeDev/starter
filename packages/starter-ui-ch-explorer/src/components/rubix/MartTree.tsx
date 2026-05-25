// PR 4 — rubix overlay. `MartTree` lists materialised marts
// surfaced by the `rubix.clickhouse.mart.list` verb and offers a
// destructive "Drop" action that delegates to
// `rubix.clickhouse.mart.drop`. Both calls go through the rubix
// verb dispatcher (`POST /api/v1/tools/{tool_id}`) — never
// directly against ClickHouse — so the snapshot-before-write +
// undo + changelog contract is preserved.
//
// The whole panel renders nothing when the verb dispatcher isn't
// mounted (HTTP 404), which is the case for the
// `examples/ch-explorer` demo binary. A rubix-agent deployment
// gets the full panel.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Boxes, Loader2, Trash2 } from "lucide-react";

import {
  callMartDrop,
  callMartList,
  RUBIX_VERB_NOT_AVAILABLE,
} from "@/api";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export function MartTree() {
  const queryClient = useQueryClient();

  const list = useQuery({
    queryKey: ["rubix-marts"],
    queryFn: callMartList,
    retry: false,
  });

  const drop = useMutation({
    mutationFn: callMartDrop,
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["rubix-marts"] }),
  });

  // Verb dispatcher not mounted → render nothing. (Loading and
  // generic-error states still render the card so the operator
  // sees the panel is wired up.)
  if (
    list.data?.ok === false &&
    list.data.status === 404 &&
    list.data.error === RUBIX_VERB_NOT_AVAILABLE
  ) {
    return null;
  }

  async function onDrop(martName: string) {
    // Mirrors the rubix admin UI's MartsPanel: a hard browser
    // confirm before any destructive call. The rubix verb itself
    // is also reversible via `rubix.undo.last`, but only an
    // operator can decide whether to invoke that.
    const ok = window.confirm(
      `DROP mart "${martName}"?\n\n` +
        "This deletes the underlying table and all its data. " +
        "You can undo via the rubix.undo.last verb, but only " +
        "until the next mutating call is recorded.",
    );
    if (!ok) return;
    await drop.mutateAsync(martName);
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">RUBIX MARTS</CardTitle>
        <Boxes className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        {list.isLoading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading marts…
          </div>
        ) : list.isError ? (
          <p className="text-sm text-red-500">
            Failed to load marts: {String(list.error)}
          </p>
        ) : list.data?.ok === false ? (
          <p className="text-sm text-red-500">
            rubix.clickhouse.mart.list failed (HTTP {list.data.status})
          </p>
        ) : list.data?.data.marts.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No marts registered yet. Use{" "}
            <code>rubix.clickhouse.mart.create</code> to provision one.
          </p>
        ) : (
          <ul className="divide-y divide-border">
            {list.data?.data.marts.map((m) => (
              <li
                key={m.mart_name}
                className="flex items-start justify-between gap-3 py-2"
              >
                <div className="min-w-0 flex-1">
                  <div className="font-mono text-sm font-medium">
                    {m.mart_name}
                  </div>
                  {m.ddl ? (
                    <pre className="mt-1 max-h-24 overflow-auto rounded-md bg-muted p-2 text-xs text-muted-foreground">
                      {m.ddl}
                    </pre>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  variant="destructive"
                  onClick={() => onDrop(m.mart_name)}
                  disabled={
                    drop.isPending && drop.variables === m.mart_name
                  }
                >
                  {drop.isPending && drop.variables === m.mart_name ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Trash2 className="h-3.5 w-3.5" />
                  )}
                  Drop
                </Button>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}
