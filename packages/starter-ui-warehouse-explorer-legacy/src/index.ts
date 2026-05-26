// @nube/starter-ui-ch-explorer — headless React component library
// for the ClickHouse explorer (forked from sql-studio, MIT).
//
// Mount `<Explorer>` inside a host that provides:
//   * `<QueryClientProvider>` from `@tanstack/react-query`
//   * `@nube/starter-ui-kit` design tokens (light/dark via the
//     `dark` class on `<html>`)
//   * For destructive surfaces (PR 2+): the rubix client + its
//     verb dispatcher at `POST /api/v1/tools/{tool_id}`.
//
// Individual views (`<ExplorerOverview />`, `<ExplorerTables />`,
// `<ExplorerSchema />`, `<ExplorerQuery />`) can also be imported
// from `./views` for hosts wanting a different layout.

export * from "./views/index.js";
export * from "./hooks/index.js";
export * from "./i18n/index.js";
