// Side-effect registration barrel for the React Native SDUI
// renderer kit. Mirrors `packages/starter-ui-sdui-react/src/renderer/index.ts`
// one-for-one (16 kinds today, in the same priority order). Each
// `render-*.tsx` calls `registerRenderer(kind, RenderX)` at module
// load against the shared registry in
// `@nube/starter-ui-sdui-react/headless`.
//
// Importing this module is the entire integration surface for a
// mobile app:
//
//   import "@nube/starter-ui-sdui-native";
//   import { SduiPage, SduiProvider } from "@nube/starter-ui-sdui-react/headless";
//
// The remaining IR kinds the web renderer does not register today
// (`stack`, `card`, `text`, `heading`, `badge`, `button`, `link`,
// `field`, `sparkline`) are deferred-with-web by design — see this
// package's README. They are NOT silently aliased here even though
// the web file aliases `sparkline` to `chart`; per spec we wait for
// an explicit web-first registration before mirroring.

import "./render-page.js";
import "./render-row.js";
import "./render-col.js";
import "./render-grid.js";
import "./render-kpi.js";
import "./render-kpi-grid.js";
import "./render-chart.js";
import "./render-divider.js";
import "./render-tabs.js";
import "./render-table.js";
import "./render-form.js";
import "./render-select.js";
import "./render-slider.js";
import "./render-toggle.js";
import "./render-date-range.js";
import "./render-repeat.js";
import "./render-custom.js";

export { RenderPage } from "./render-page.js";
export { RenderRow } from "./render-row.js";
export { RenderCol } from "./render-col.js";
export { RenderGrid } from "./render-grid.js";
export { RenderKpi } from "./render-kpi.js";
export { RenderKpiGrid } from "./render-kpi-grid.js";
export { RenderChart } from "./render-chart.js";
export { RenderDivider } from "./render-divider.js";
export { RenderTabs } from "./render-tabs.js";
export { RenderTable } from "./render-table.js";
export { RenderForm } from "./render-form.js";
export { RenderSelect } from "./render-select.js";
export { RenderSlider } from "./render-slider.js";
export { RenderToggle } from "./render-toggle.js";
export { RenderDateRange } from "./render-date-range.js";
export { RenderRepeat } from "./render-repeat.js";
export { RenderCustom } from "./render-custom.js";
