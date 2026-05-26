// Web-renderer side-effect barrel. Each `render-*.tsx` calls
// `registerRenderer(...)` at module load against the registry in
// `../headless/registry.js`. Importing this barrel (from the root
// `src/index.ts`) wires the web kit into the shared dispatcher.
//
// Mobile builds MUST NOT import this barrel: they ship their own
// renderer side-effect file against `@nube/starter-ui-kit-native`
// and consume `@nube/starter-ui-sdui-react/headless` for the rest.

import "./render-page.js";
import "./render-row.js";
import "./render-col.js";
import "./render-grid.js";
import "./render-kpi.js";
import "./render-kpi-grid.js";
import "./render-chart.js";
import "./render-table.js";
import "./render-form.js";
import "./render-tabs.js";
import "./render-select.js";
import "./render-slider.js";
import "./render-toggle.js";
import "./render-date-range.js";
import "./render-divider.js";
import "./render-custom.js";
import "./render-repeat.js";

export { RenderPage } from "./render-page.js";
export { RenderRow } from "./render-row.js";
export { RenderCol } from "./render-col.js";
export { RenderGrid } from "./render-grid.js";
export { RenderKpi } from "./render-kpi.js";
export { RenderKpiGrid } from "./render-kpi-grid.js";
export { RenderChart } from "./render-chart.js";
export { RenderTable } from "./render-table.js";
export { RenderForm } from "./render-form.js";
export { RenderTabs } from "./render-tabs.js";
export { RenderSelect } from "./render-select.js";
export { RenderSlider } from "./render-slider.js";
export { RenderToggle } from "./render-toggle.js";
export { RenderDateRange } from "./render-date-range.js";
export { RenderDivider } from "./render-divider.js";
export { RenderCustom } from "./render-custom.js";
export { RenderRepeat } from "./render-repeat.js";
