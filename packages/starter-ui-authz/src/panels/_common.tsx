// Small primitives shared by every panel: state rows (loading,
// empty, error) and a thin wrapper over the shadcn `Table`
// primitive so each panel renders rows without redeclaring the
// header/cell styling.

import { type ReactNode } from "react";
import {
  Table,
  TableBody,
  TableHead,
  TableHeader,
  TableRow,
  cn,
} from "@nube/starter-ui-kit";
import { AlertCircle, Loader2, Inbox } from "lucide-react";

export interface StateRowProps {
  /** "loading" | "empty" | "error". */
  variant: "loading" | "empty" | "error";
  children: ReactNode;
}

/** Centered row inside a card or table. Uses the kit's
 * `text-muted-foreground` / `text-destructive` tokens so it
 * stays legible across light + dark themes. */
export function StateRow({ variant, children }: StateRowProps) {
  const Icon =
    variant === "loading" ? Loader2 : variant === "empty" ? Inbox : AlertCircle;
  return (
    <div
      className={cn(
        "flex items-center justify-center gap-2 rounded-2xl border border-dashed border-border bg-muted/30 px-4 py-10 text-sm",
        variant === "error" ? "text-destructive" : "text-muted-foreground",
      )}
      role={variant === "error" ? "alert" : undefined}
    >
      <Icon
        className={cn(
          "size-4 shrink-0",
          variant === "loading" && "animate-spin",
        )}
        aria-hidden
      />
      <span>{children}</span>
    </div>
  );
}

export interface DataTableProps {
  /** Header row labels. */
  headers: ReactNode[];
  /** Body rows. Caller renders the `<td>`s — use the kit's
   * `TableCell` / `TableRow` for cell-level styling. */
  rows: ReactNode[];
  /** `aria-label` for the table. */
  label?: string;
}

/** Bordered, rounded table card built on shadcn `Table`. The
 * outer wrapper handles overflow so wide tables (Resources,
 * Decisions) scroll horizontally inside the card instead of
 * blowing past the parent's right edge. */
export function DataTable({ headers, rows, label }: DataTableProps) {
  return (
    <div className="overflow-hidden rounded-2xl border border-border bg-card">
      <Table aria-label={label}>
        <TableHeader>
          <TableRow className="bg-muted/40 hover:bg-muted/40">
            {headers.map((h, i) => (
              <TableHead key={i}>{h}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>{rows}</TableBody>
      </Table>
    </div>
  );
}

/** Standard body cell. Re-exported from the kit primitive but
 * with the legacy `Td` name so existing panel code keeps
 * compiling. New panels should import `TableCell` directly. */
export { TableCell as Td } from "@nube/starter-ui-kit";

/** A right-aligned cell for row-level actions. */
export function ActionsCell({ children }: { children?: ReactNode }) {
  return (
    <td className="px-4 py-3 text-right align-middle">
      <div className="flex justify-end gap-2">{children}</div>
    </td>
  );
}
