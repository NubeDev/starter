// `page-preview-tab.tsx` — pick a page and render its widgets via PageView.
import * as React from "react";
import { listPages } from "../bc-api";
import type { PageRow } from "../bc-types";
import { PageView } from "../page-render/page-view";
import { Select } from "../ui/select";

export function PagePreviewTab(): React.ReactElement {
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const [pageId, setPageId] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    listPages()
      .then((ps) => {
        setPages(ps);
        if (ps[0]) setPageId(ps[0].page_id);
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <div className="flex flex-col gap-4">
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      <div className="max-w-xs">
        <Select
          label="Page"
          value={pageId}
          placeholder="Select a page"
          options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
          onChange={setPageId}
        />
      </div>
      <PageView pageId={pageId} />
    </div>
  );
}
