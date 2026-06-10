import { useState } from "react";
import { ChevronRight } from "lucide-react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@nube/starter-ui-kit/components/table";
import { cn } from "@nube/starter-ui-kit/lib/utils";

import type { Change } from "@/api/types";
import { useDateTime } from "@/datetime";
import { actorLabel, opLabel } from "@/features/audit/actorLabel";
import { ChangeDiff } from "@/features/audit/ChangeDiff";

// A newest-first table of changes, one compact row per change (who/what/when),
// with an expandable before -> after diff so the ledger stays scannable even
// with hundreds of entries. Shared by the admin audit screen and a per-resource
// History tab. Empty/loading/error are the caller's concern (F0).
export function ChangeList({ changes }: { changes: Change[] }) {
  if (changes.length === 0) {
    return (
      <p className="py-8 text-center text-sm text-muted-foreground">
        No changes recorded yet.
      </p>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-8" />
          <TableHead>Operation</TableHead>
          <TableHead>Resource</TableHead>
          <TableHead>Actor</TableHead>
          <TableHead className="text-right">When</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {changes.map((change) => (
          <ChangeRow key={change.id} change={change} />
        ))}
      </TableBody>
    </Table>
  );
}

function ChangeRow({ change }: { change: Change }) {
  const { dateTime } = useDateTime();
  const [open, setOpen] = useState(false);

  return (
    <>
      <TableRow
        className="cursor-pointer"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <TableCell className="text-muted-foreground">
          <ChevronRight
            className={cn(
              "size-4 transition-transform",
              open && "rotate-90",
            )}
          />
        </TableCell>
        <TableCell>
          <Badge variant="secondary">{opLabel(change.op)}</Badge>
        </TableCell>
        <TableCell>
          <div className="flex flex-col">
            <span className="font-medium">{change.resource.kind}</span>
            {change.resource.id ? (
              <span className="font-mono text-xs text-muted-foreground">
                {change.resource.id}
              </span>
            ) : null}
          </div>
        </TableCell>
        <TableCell className="text-muted-foreground">
          {actorLabel(change.actor)}
        </TableCell>
        <TableCell className="text-right text-xs text-muted-foreground">
          {dateTime(change.at)}
        </TableCell>
      </TableRow>
      {open ? (
        <TableRow className="hover:bg-transparent">
          <TableCell colSpan={5} className="bg-muted/30">
            <ChangeDiff
              before={change.before as Record<string, unknown> | null}
              after={change.after as Record<string, unknown> | null}
            />
          </TableCell>
        </TableRow>
      ) : null}
    </>
  );
}
