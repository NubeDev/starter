// Small primitives shared by every panel: a centered loading
// row, an empty-state row, an error row, and a minimal HTML
// table styled to feel native next to `@nube/starter-ui-kit`.

import { type ReactNode } from "react";
import clsx from "clsx";

export interface StateRowProps {
  /** "loading" | "empty" | "error". */
  variant: "loading" | "empty" | "error";
  children: ReactNode;
}

/** Centered row inside a card or table. */
export function StateRow({ variant, children }: StateRowProps) {
  return (
    <div
      className={clsx(
        "flex items-center justify-center px-4 py-10 text-sm",
        variant === "error"
          ? "text-[color:var(--color-danger,#dc2626)]"
          : "text-[color:var(--color-subtle,#6b7280)]",
      )}
      role={variant === "error" ? "alert" : undefined}
    >
      {children}
    </div>
  );
}

export interface DataTableProps {
  /** Header row labels. */
  headers: ReactNode[];
  /** Body rows. Caller renders the `<td>`s. */
  rows: ReactNode[];
  /** `aria-label` for the table. */
  label?: string;
}

/** Minimal accessible table. Caller is responsible for cell
 * content and per-row actions. */
export function DataTable({ headers, rows, label }: DataTableProps) {
  return (
    <div className="overflow-hidden rounded-2xl border border-[color:var(--color-border,#e5e7eb)]">
      <table className="w-full border-collapse text-sm" aria-label={label}>
        <thead className="bg-[color:var(--color-muted,#f9fafb)]">
          <tr>
            {headers.map((h, i) => (
              <th
                key={i}
                className="border-b border-[color:var(--color-border,#e5e7eb)] px-4 py-2 text-left text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle,#6b7280)]"
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}

/** Standard body cell. */
export function Td({ children, className }: { children?: ReactNode; className?: string }) {
  return (
    <td className={clsx("border-b border-[color:var(--color-border,#e5e7eb)] px-4 py-2 align-middle", className)}>
      {children}
    </td>
  );
}

/** A right-aligned cell for row-level actions. */
export function ActionsCell({ children }: { children?: ReactNode }) {
  return (
    <Td className="text-right">
      <div className="flex justify-end gap-2">{children}</div>
    </Td>
  );
}
