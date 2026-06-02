// `place-on-page.tsx` — inline picker for placing a pending device on a
// dashboard page. Lets the user pick an existing page (scoped to the
// device's site) or type a new page name, then calls assignPage (a
// mutation → bumps refresh so the devices list converges).
import * as React from "react";
import { Check, X } from "lucide-react";
import { assignPage, listPages } from "../bc-api";
import type { DeviceRow, PageRow } from "../bc-types";

export function PlaceOnPage({
  device,
  onClose,
}: {
  device: DeviceRow;
  onClose: () => void;
}): React.ReactElement {
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const [pageId, setPageId] = React.useState("");
  const [newPage, setNewPage] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    listPages({ site_id: device.site_id ?? undefined })
      .then(setPages)
      .catch(() => setPages([]));
  }, [device.site_id]);

  const ready = !!pageId || !!newPage.trim();

  const submit = () => {
    if (!ready || busy) return;
    setBusy(true);
    setError(null);
    assignPage(
      pageId
        ? { device_id: device.device_id, page_id: pageId }
        : { device_id: device.device_id, new_page: { name: newPage.trim() } },
    )
      // assignPage bumps refresh, so the devices list re-fetches and the
      // row drops out of "pending" on its own — we just close the picker.
      .then(() => onClose())
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border/60 bg-background p-3">
      <div className="flex items-center justify-between">
        <span className="ext-eyebrow">Place {device.name ?? device.device_id} on a page</span>
        <button
          type="button"
          aria-label="Cancel"
          onClick={onClose}
          className="cursor-pointer rounded-md p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <X className="size-4" />
        </button>
      </div>

      <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
        <label className="flex flex-1 flex-col gap-1.5">
          <span className="ext-eyebrow">Existing page</span>
          <select
            value={pageId}
            onChange={(e) => { setPageId(e.target.value); setNewPage(""); }}
            aria-label="Existing page"
            className="cursor-pointer rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-border focus:border-primary focus:outline-none"
          >
            <option value="">+ New page</option>
            {pages.map((p) => (
              <option key={p.page_id} value={p.page_id}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        {!pageId ? (
          <label className="flex flex-1 flex-col gap-1.5">
            <span className="ext-eyebrow">New page name</span>
            <input
              value={newPage}
              onChange={(e) => setNewPage(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              placeholder="e.g. Floor 3"
              aria-label="New page name"
              className="rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
            />
          </label>
        ) : null}
        <button
          type="button"
          onClick={submit}
          disabled={busy || !ready}
          className="flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Check className="size-4" /> {busy ? "Placing…" : "Place"}
        </button>
      </div>

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {error}
        </div>
      ) : null}
    </div>
  );
}
