// Rubix overlay — lists materialised marts surfaced by the
// `rubix.clickhouse.mart.list` verb and offers a destructive
// "Drop" action that delegates to `rubix.clickhouse.mart.drop`.
// Both calls go through the typed `useClickhouseMartsList` /
// `useClickhouseMartDrop` hooks from `@nube/rubix-client-react`
// so the snapshot-before-write + undo + changelog contract is
// preserved.
//
// Destructive confirmation uses `<AlertDialog>` from the kit
// (never `window.confirm`).
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useState } from "react";
import {
  useClickhouseMartDrop,
  useClickhouseMartsList,
} from "@nube/rubix-client-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Skeleton,
} from "@nube/starter-ui-kit";
import { AlertTriangle, Boxes, Trash2 } from "lucide-react";

export interface MartTreeMessages {
  /** Section header title. */
  title: string;
  /** Empty-state title. */
  emptyTitle: string;
  /** Empty-state description. */
  emptyDescription: string;
  /** Generic load error. */
  loadError: string;
  /** Drop button label. */
  drop: string;
  /** Confirm-dialog title. `{name}` is replaced with the mart name. */
  confirmTitle: string;
  /** Confirm-dialog description. */
  confirmDescription: string;
  /** Confirm-dialog destructive action label. */
  confirmAction: string;
  /** Confirm-dialog cancel label. */
  confirmCancel: string;
}

const DEFAULT_MART_TREE_MESSAGES: MartTreeMessages = {
  title: "RUBIX MARTS",
  emptyTitle: "No marts",
  emptyDescription:
    "Create a mart with the rubix.clickhouse.mart.create verb to materialise an L1–L3 aggregate.",
  loadError: "Failed to load marts.",
  drop: "Drop",
  confirmTitle: 'DROP mart "{name}"?',
  confirmDescription:
    "This deletes the underlying table and all its data. The operation is reversible via rubix.undo.last, but only until the next mutating call is recorded.",
  confirmAction: "Drop mart",
  confirmCancel: "Cancel",
};

export interface MartTreeProps {
  /** Optional message override. Defaults to English. */
  messages?: Partial<MartTreeMessages>;
}

export function MartTree({ messages }: MartTreeProps = {}) {
  const m: MartTreeMessages = { ...DEFAULT_MART_TREE_MESSAGES, ...messages };
  const list = useClickhouseMartsList({ retry: false });
  const drop = useClickhouseMartDrop();
  const [pending, setPending] = useState<string | null>(null);

  const rows = list.data?.marts ?? [];

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{m.title}</CardTitle>
        <Boxes className="h-4 w-4 text-[color:var(--color-muted)]" />
      </CardHeader>
      <CardContent>
        {list.isPending ? (
          <div className="space-y-2">
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
          </div>
        ) : list.isError ? (
          <p className="text-sm text-red-500">{m.loadError}</p>
        ) : rows.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Boxes />
              </EmptyMedia>
              <EmptyTitle>{m.emptyTitle}</EmptyTitle>
              <EmptyDescription>{m.emptyDescription}</EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          <ul className="divide-y divide-[color:var(--color-border)]">
            {rows.map((row) => (
              <li
                key={row.mart_name}
                className="flex items-start justify-between gap-3 py-2"
              >
                <div className="min-w-0 flex-1">
                  <div className="font-mono text-sm font-medium">
                    {row.mart_name}
                  </div>
                  {row.ddl ? (
                    <pre className="mt-1 max-h-24 overflow-auto rounded-lg bg-[color:var(--color-surface-2)] p-2 text-xs text-[color:var(--color-muted)]">
                      {row.ddl}
                    </pre>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  variant="destructive"
                  onClick={() => setPending(row.mart_name)}
                  disabled={drop.isPending && drop.variables?.mart_name === row.mart_name}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                  {m.drop}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </CardContent>

      <AlertDialog
        open={pending !== null}
        onOpenChange={(open) => !open && setPending(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              {m.confirmTitle.replace("{name}", pending ?? "")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {m.confirmDescription}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{m.confirmCancel}</AlertDialogCancel>
            <AlertDialogAction
              onClick={async () => {
                const name = pending;
                setPending(null);
                if (!name) return;
                await drop.mutateAsync({ mart_name: name });
              }}
            >
              {m.confirmAction}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
