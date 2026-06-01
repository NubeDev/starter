// `page-preview-tab.tsx` — the client view: pick a Site, then one of
// that site's dashboard pages, and render its widgets. Mirrors how an
// end user browses "Building A → its dashboards → the sensors on it".
import * as React from "react";
import { listPages, listSites } from "../bc-api";
import { useRefreshKey } from "../refresh";
import type { PageRow, SiteRow } from "../bc-types";
import { PageView } from "../page-render/page-view";
import { Select } from "../ui/select";

export function PagePreviewTab(): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [siteId, setSiteId] = React.useState("");
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const [pageId, setPageId] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);
  const refresh = useRefreshKey();

  React.useEffect(() => {
    listSites()
      .then((ss) => {
        setSites(ss);
        setSiteId((cur) => (cur && ss.some((s) => s.site_id === cur) ? cur : ss[0]?.site_id ?? ""));
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, [refresh]);

  React.useEffect(() => {
    if (!siteId) {
      setPages([]);
      setPageId("");
      return;
    }
    listPages({ site_id: siteId })
      .then((ps) => {
        setPages(ps);
        setPageId((cur) => (cur && ps.some((p) => p.page_id === cur) ? cur : ps[0]?.page_id ?? ""));
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, [siteId, refresh]);

  return (
    <div className="flex flex-col gap-4">
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      <div className="flex flex-wrap gap-3">
        <div className="min-w-48">
          <Select
            label="Site"
            value={siteId}
            placeholder="Select a site"
            options={sites.map((s) => ({ value: s.site_id, label: s.name }))}
            onChange={setSiteId}
          />
        </div>
        <div className="min-w-48">
          <Select
            label="Dashboard page"
            value={pageId}
            placeholder={pages.length ? "Select a page" : "No pages for this site yet"}
            options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
            onChange={setPageId}
          />
        </div>
      </div>
      <PageView pageId={pageId} />
    </div>
  );
}
