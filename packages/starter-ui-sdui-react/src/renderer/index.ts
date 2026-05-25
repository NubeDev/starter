// Import side-effect: every render-*.tsx file calls
// `registerRenderer(...)` at module load. Importing this barrel once
// (from `src/index.ts` or `src/sdui-page.tsx`) wires up the registry.

import "./render-page.js";
import "./render-row.js";
import "./render-col.js";
import "./render-grid.js";
import "./render-kpi.js";
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

export { Render, RenderChildren } from "./render.js";
export {
  registerRenderer,
  lookupRenderer,
  listRenderers,
} from "./registry.js";
export type { Renderer, RenderProps, CustomRendererRegistry } from "./registry.js";
export { RenderPage } from "./render-page.js";
export { RenderRow } from "./render-row.js";
export { RenderCol } from "./render-col.js";
export { RenderGrid } from "./render-grid.js";
export { RenderKpi } from "./render-kpi.js";
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
