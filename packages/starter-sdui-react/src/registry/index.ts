/**
 * Built-in component registry barrel.
 *
 * Each `components/<X>.tsx` exports a `<x>Spec: ComponentSpec<XNode>`;
 * this barrel collects them into `builtinComponentRegistry` — the
 * one map the `Renderer` consumes. Adding a new built-in kind is:
 * export a spec, add it to `collectSpecs()`, done. Drift against
 * `Kind` is a TS error.
 *
 * Custom (non-built-in) kinds plug in via `registerCustomRenderer`
 * (see `../context.ts`); they live in `globalCustomRegistry` and the
 * `custom` built-in dispatches against it at render time.
 */
import type { ComponentRegistry, ComponentSpec, Kind } from "./types.js";
import { pageSpec } from "../components/Page.js";
import { rowSpec, colSpec, gridSpec, stackSpec } from "../components/Layout.js";
import { tabsSpec } from "../components/Tabs.js";
import { cardSpec } from "../components/Card.js";
import { textSpec, headingSpec, badgeSpec } from "../components/Display.js";
import { kpiSpec, kpiGridSpec } from "../components/Kpi.js";
import { buttonSpec, linkSpec } from "../components/Interactive.js";
import { tableSpec } from "../components/Table.js";
import { formSpec } from "../components/Form.js";
import { fieldSpec, selectSpec, toggleSpec } from "../components/Inputs.js";
import { customSpec } from "../components/Custom.js";
import { chartSpec, sparklineSpec } from "../components/Chart.js";
import { treeSpec } from "../components/Tree.js";
import {
  markdownSpec,
  codeSpec,
  timelineSpec,
} from "../components/Streaming.js";
import { wizardSpec, drawerSpec } from "../components/Wizard.js";
import { richTextSpec } from "../components/RichText.js";
import { diffSpec } from "../components/Diff.js";
import { refPickerSpec, dateRangeSpec } from "../components/Pickers.js";

export type { ComponentRegistry, ComponentSpec, Kind } from "./types.js";

// Lazy build — every spec import transitively pulls `Renderer.tsx`,
// which imports back into this barrel. The cycle is benign at runtime
// because `lookupSpec` is called from inside React render, well after
// every module body has finished. But evaluating
// `Object.fromEntries(specs.map(...))` at module-load time blows up
// when a leaf component file is the entry point of the module graph
// (vitest does this when a single component test is loaded): its
// named export is still `undefined` while its body is in flight, so
// `s.kind` throws.
//
// Deferring the build to first access fixes the bootstrap race; the
// result is memoised so consumers that iterate
// (`Object.values(builtinComponentRegistry)`) keep their O(1) reads.
function collectSpecs(): ReadonlyArray<ComponentSpec<unknown>> {
  return [
    pageSpec, rowSpec, colSpec, gridSpec, stackSpec,
    tabsSpec, cardSpec,
    textSpec, headingSpec, badgeSpec,
    kpiSpec, kpiGridSpec,
    buttonSpec, linkSpec,
    tableSpec, formSpec,
    fieldSpec, selectSpec, toggleSpec,
    customSpec,
    // Phase 6 — remaining IR variants.
    chartSpec, sparklineSpec,
    treeSpec, timelineSpec,
    markdownSpec, codeSpec,
    wizardSpec, drawerSpec,
    richTextSpec, diffSpec,
    refPickerSpec, dateRangeSpec,
  ] as unknown as ReadonlyArray<ComponentSpec<unknown>>;
}

let _registry: ComponentRegistry | null = null;
function buildRegistry(): ComponentRegistry {
  return Object.freeze(
    Object.fromEntries(collectSpecs().map((s) => [s.kind, s])),
  ) as ComponentRegistry;
}

export const builtinComponentRegistry: ComponentRegistry = new Proxy(
  {} as ComponentRegistry,
  {
    get(_t, prop) {
      if (!_registry) _registry = buildRegistry();
      return Reflect.get(_registry, prop);
    },
    has(_t, prop) {
      if (!_registry) _registry = buildRegistry();
      return Reflect.has(_registry, prop);
    },
    ownKeys() {
      if (!_registry) _registry = buildRegistry();
      return Reflect.ownKeys(_registry);
    },
    getOwnPropertyDescriptor(_t, prop) {
      if (!_registry) _registry = buildRegistry();
      const d = Reflect.getOwnPropertyDescriptor(_registry, prop);
      if (!d) return undefined;
      return { ...d, configurable: true };
    },
  },
);

/**
 * Look up a built-in spec by kind. Returns `undefined` for unknown
 * kinds; callers fall back to the custom-renderer registry, then to
 * the unknown-component placeholder.
 */
export function lookupSpec(kind: string): ComponentSpec<unknown> | undefined {
  return (builtinComponentRegistry as Record<string, ComponentSpec<unknown> | undefined>)[kind];
}

export type { Kind as BuiltinKind };
