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
// Content / page-builder blocks (V2.0).
import "./render-section.js";
import "./render-card.js";
import "./render-hero.js";
import "./render-image.js";
import "./render-spacer.js";
import "./render-text.js";
import "./render-heading.js";
import "./render-button.js";
import "./render-rich-text.js";

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
export { RenderSection } from "./render-section.js";
export { RenderCard } from "./render-card.js";
export { RenderHero } from "./render-hero.js";
export { RenderImage } from "./render-image.js";
export { RenderSpacer } from "./render-spacer.js";
export { RenderText } from "./render-text.js";
export { RenderHeading } from "./render-heading.js";
export { RenderButton } from "./render-button.js";
export { RenderRichText } from "./render-rich-text.js";
