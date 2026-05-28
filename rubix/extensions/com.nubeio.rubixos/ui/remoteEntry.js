(function () {
	'use strict';

	try{if(typeof document != 'undefined'){var elementStyle = document.createElement('style');elementStyle.appendChild(document.createTextNode("/*! tailwindcss v4.3.0 | MIT License | https://tailwindcss.com */\n@layer properties {\n  @supports (((-webkit-hyphens: none)) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color: rgb(from red r g b)))) {\n    *, :before, :after, ::backdrop {\n      --tw-space-y-reverse: 0;\n      --tw-border-style: solid;\n      --tw-font-weight: initial;\n      --tw-tracking: initial;\n      --tw-ordinal: initial;\n      --tw-slashed-zero: initial;\n      --tw-numeric-figure: initial;\n      --tw-numeric-spacing: initial;\n      --tw-numeric-fraction: initial;\n      --tw-duration: initial;\n    }\n  }\n}\n\n@layer theme {\n  :root, :host {\n    --font-sans: ui-sans-serif, system-ui, sans-serif, \"Apple Color Emoji\",\n      \"Segoe UI Emoji\", \"Segoe UI Symbol\", \"Noto Color Emoji\";\n    --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\",\n      \"Courier New\", monospace;\n    --spacing: .25rem;\n    --text-xs: .75rem;\n    --text-xs--line-height: calc(1 / .75);\n    --text-sm: .875rem;\n    --text-sm--line-height: calc(1.25 / .875);\n    --text-lg: 1.125rem;\n    --text-lg--line-height: calc(1.75 / 1.125);\n    --text-xl: 1.25rem;\n    --text-xl--line-height: calc(1.75 / 1.25);\n    --font-weight-normal: 400;\n    --font-weight-medium: 500;\n    --font-weight-semibold: 600;\n    --tracking-tight: -.025em;\n    --tracking-wide: .025em;\n    --tracking-wider: .05em;\n    --radius-md: .375rem;\n    --radius-lg: .5rem;\n    --default-transition-duration: .15s;\n    --default-transition-timing-function: cubic-bezier(.4, 0, .2, 1);\n    --default-font-family: var(--font-sans);\n    --default-mono-font-family: var(--font-mono);\n  }\n}\n\n@layer base {\n  *, :after, :before, ::backdrop {\n    box-sizing: border-box;\n    border: 0 solid;\n    margin: 0;\n    padding: 0;\n  }\n\n  ::file-selector-button {\n    box-sizing: border-box;\n    border: 0 solid;\n    margin: 0;\n    padding: 0;\n  }\n\n  html, :host {\n    -webkit-text-size-adjust: 100%;\n    tab-size: 4;\n    line-height: 1.5;\n    font-family: var(--default-font-family, ui-sans-serif, system-ui, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Segoe UI Symbol\", \"Noto Color Emoji\");\n    font-feature-settings: var(--default-font-feature-settings, normal);\n    font-variation-settings: var(--default-font-variation-settings, normal);\n    -webkit-tap-highlight-color: transparent;\n  }\n\n  hr {\n    height: 0;\n    color: inherit;\n    border-top-width: 1px;\n  }\n\n  abbr:where([title]) {\n    -webkit-text-decoration: underline dotted;\n    text-decoration: underline dotted;\n  }\n\n  h1, h2, h3, h4, h5, h6 {\n    font-size: inherit;\n    font-weight: inherit;\n  }\n\n  a {\n    color: inherit;\n    -webkit-text-decoration: inherit;\n    -webkit-text-decoration: inherit;\n    -webkit-text-decoration: inherit;\n    text-decoration: inherit;\n  }\n\n  b, strong {\n    font-weight: bolder;\n  }\n\n  code, kbd, samp, pre {\n    font-family: var(--default-mono-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace);\n    font-feature-settings: var(--default-mono-font-feature-settings, normal);\n    font-variation-settings: var(--default-mono-font-variation-settings, normal);\n    font-size: 1em;\n  }\n\n  small {\n    font-size: 80%;\n  }\n\n  sub, sup {\n    vertical-align: baseline;\n    font-size: 75%;\n    line-height: 0;\n    position: relative;\n  }\n\n  sub {\n    bottom: -.25em;\n  }\n\n  sup {\n    top: -.5em;\n  }\n\n  table {\n    text-indent: 0;\n    border-color: inherit;\n    border-collapse: collapse;\n  }\n\n  :-moz-focusring {\n    outline: auto;\n  }\n\n  progress {\n    vertical-align: baseline;\n  }\n\n  summary {\n    display: list-item;\n  }\n\n  ol, ul, menu {\n    list-style: none;\n  }\n\n  img, svg, video, canvas, audio, iframe, embed, object {\n    vertical-align: middle;\n    display: block;\n  }\n\n  img, video {\n    max-width: 100%;\n    height: auto;\n  }\n\n  button, input, select, optgroup, textarea {\n    font: inherit;\n    font-feature-settings: inherit;\n    font-variation-settings: inherit;\n    letter-spacing: inherit;\n    color: inherit;\n    opacity: 1;\n    background-color: #0000;\n    border-radius: 0;\n  }\n\n  ::file-selector-button {\n    font: inherit;\n    font-feature-settings: inherit;\n    font-variation-settings: inherit;\n    letter-spacing: inherit;\n    color: inherit;\n    opacity: 1;\n    background-color: #0000;\n    border-radius: 0;\n  }\n\n  :where(select:is([multiple], [size])) optgroup {\n    font-weight: bolder;\n  }\n\n  :where(select:is([multiple], [size])) optgroup option {\n    padding-inline-start: 20px;\n  }\n\n  ::file-selector-button {\n    margin-inline-end: 4px;\n  }\n\n  ::placeholder {\n    opacity: 1;\n  }\n\n  @supports (not ((-webkit-appearance: -apple-pay-button))) or (contain-intrinsic-size: 1px) {\n    ::placeholder {\n      color: currentColor;\n    }\n\n    @supports (color: color-mix(in lab, red, red)) {\n      ::placeholder {\n        color: color-mix(in oklab, currentcolor 50%, transparent);\n      }\n    }\n  }\n\n  textarea {\n    resize: vertical;\n  }\n\n  ::-webkit-search-decoration {\n    -webkit-appearance: none;\n  }\n\n  ::-webkit-date-and-time-value {\n    min-height: 1lh;\n    text-align: inherit;\n  }\n\n  ::-webkit-datetime-edit {\n    display: inline-flex;\n  }\n\n  ::-webkit-datetime-edit-fields-wrapper {\n    padding: 0;\n  }\n\n  ::-webkit-datetime-edit {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-year-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-month-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-day-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-hour-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-minute-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-second-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-millisecond-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-datetime-edit-meridiem-field {\n    padding-block: 0;\n  }\n\n  ::-webkit-calendar-picker-indicator {\n    line-height: 1;\n  }\n\n  :-moz-ui-invalid {\n    box-shadow: none;\n  }\n\n  button, input:where([type=\"button\"], [type=\"reset\"], [type=\"submit\"]) {\n    appearance: button;\n  }\n\n  ::file-selector-button {\n    appearance: button;\n  }\n\n  ::-webkit-inner-spin-button {\n    height: auto;\n  }\n\n  ::-webkit-outer-spin-button {\n    height: auto;\n  }\n\n  [hidden]:where(:not([hidden=\"until-found\"])) {\n    display: none !important;\n  }\n}\n\n@layer components;\n\n@layer utilities {\n  .m-0 {\n    margin: calc(var(--spacing) * 0);\n  }\n\n  .-mx-1 {\n    margin-inline: calc(var(--spacing) * -1);\n  }\n\n  .mx-2 {\n    margin-inline: calc(var(--spacing) * 2);\n  }\n\n  .my-1 {\n    margin-block: calc(var(--spacing) * 1);\n  }\n\n  .mt-1 {\n    margin-top: calc(var(--spacing) * 1);\n  }\n\n  .mt-2 {\n    margin-top: calc(var(--spacing) * 2);\n  }\n\n  .mt-3 {\n    margin-top: calc(var(--spacing) * 3);\n  }\n\n  .mb-3 {\n    margin-bottom: calc(var(--spacing) * 3);\n  }\n\n  .ml-2 {\n    margin-left: calc(var(--spacing) * 2);\n  }\n\n  .ml-3 {\n    margin-left: calc(var(--spacing) * 3);\n  }\n\n  .ml-4 {\n    margin-left: calc(var(--spacing) * 4);\n  }\n\n  .block {\n    display: block;\n  }\n\n  .flex {\n    display: flex;\n  }\n\n  .grid {\n    display: grid;\n  }\n\n  .inline-block {\n    display: inline-block;\n  }\n\n  .max-h-\\[60vh\\] {\n    max-height: 60vh;\n  }\n\n  .w-44 {\n    width: calc(var(--spacing) * 44);\n  }\n\n  .w-full {\n    width: 100%;\n  }\n\n  .shrink-0 {\n    flex-shrink: 0;\n  }\n\n  .rotate-90 {\n    rotate: 90deg;\n  }\n\n  .cursor-pointer {\n    cursor: pointer;\n  }\n\n  .list-none {\n    list-style-type: none;\n  }\n\n  .grid-cols-1 {\n    grid-template-columns: repeat(1, minmax(0, 1fr));\n  }\n\n  .grid-cols-2 {\n    grid-template-columns: repeat(2, minmax(0, 1fr));\n  }\n\n  .grid-cols-4 {\n    grid-template-columns: repeat(4, minmax(0, 1fr));\n  }\n\n  .flex-col {\n    flex-direction: column;\n  }\n\n  .flex-wrap {\n    flex-wrap: wrap;\n  }\n\n  .items-baseline {\n    align-items: baseline;\n  }\n\n  .items-center {\n    align-items: center;\n  }\n\n  .justify-between {\n    justify-content: space-between;\n  }\n\n  .gap-1 {\n    gap: calc(var(--spacing) * 1);\n  }\n\n  .gap-1\\.5 {\n    gap: calc(var(--spacing) * 1.5);\n  }\n\n  .gap-2 {\n    gap: calc(var(--spacing) * 2);\n  }\n\n  .gap-3 {\n    gap: calc(var(--spacing) * 3);\n  }\n\n  .gap-4 {\n    gap: calc(var(--spacing) * 4);\n  }\n\n  :where(.space-y-0\\.5 > :not(:last-child)) {\n    --tw-space-y-reverse: 0;\n    margin-block-start: calc(calc(var(--spacing) * .5) * var(--tw-space-y-reverse));\n    margin-block-end: calc(calc(var(--spacing) * .5) * calc(1 - var(--tw-space-y-reverse)));\n  }\n\n  .truncate {\n    text-overflow: ellipsis;\n    white-space: nowrap;\n    overflow: hidden;\n  }\n\n  .overflow-x-auto {\n    overflow-x: auto;\n  }\n\n  .overflow-y-auto {\n    overflow-y: auto;\n  }\n\n  .rounded {\n    border-radius: .25rem;\n  }\n\n  .rounded-lg {\n    border-radius: var(--radius-lg);\n  }\n\n  .rounded-md {\n    border-radius: var(--radius-md);\n  }\n\n  .border {\n    border-style: var(--tw-border-style);\n    border-width: 1px;\n  }\n\n  .border-0 {\n    border-style: var(--tw-border-style);\n    border-width: 0;\n  }\n\n  .border-t {\n    border-top-style: var(--tw-border-style);\n    border-top-width: 1px;\n  }\n\n  .border-b {\n    border-bottom-style: var(--tw-border-style);\n    border-bottom-width: 1px;\n  }\n\n  .border-l {\n    border-left-style: var(--tw-border-style);\n    border-left-width: 1px;\n  }\n\n  .bg-transparent {\n    background-color: #0000;\n  }\n\n  .p-0 {\n    padding: calc(var(--spacing) * 0);\n  }\n\n  .p-3 {\n    padding: calc(var(--spacing) * 3);\n  }\n\n  .p-4 {\n    padding: calc(var(--spacing) * 4);\n  }\n\n  .px-1\\.5 {\n    padding-inline: calc(var(--spacing) * 1.5);\n  }\n\n  .px-2 {\n    padding-inline: calc(var(--spacing) * 2);\n  }\n\n  .px-3 {\n    padding-inline: calc(var(--spacing) * 3);\n  }\n\n  .py-0\\.5 {\n    padding-block: calc(var(--spacing) * .5);\n  }\n\n  .py-1 {\n    padding-block: calc(var(--spacing) * 1);\n  }\n\n  .py-1\\.5 {\n    padding-block: calc(var(--spacing) * 1.5);\n  }\n\n  .py-2 {\n    padding-block: calc(var(--spacing) * 2);\n  }\n\n  .pl-4 {\n    padding-left: calc(var(--spacing) * 4);\n  }\n\n  .pl-5 {\n    padding-left: calc(var(--spacing) * 5);\n  }\n\n  .text-left {\n    text-align: left;\n  }\n\n  .text-right {\n    text-align: right;\n  }\n\n  .font-mono {\n    font-family: var(--font-mono);\n  }\n\n  .text-lg {\n    font-size: var(--text-lg);\n    line-height: var(--tw-leading, var(--text-lg--line-height));\n  }\n\n  .text-sm {\n    font-size: var(--text-sm);\n    line-height: var(--tw-leading, var(--text-sm--line-height));\n  }\n\n  .text-xl {\n    font-size: var(--text-xl);\n    line-height: var(--tw-leading, var(--text-xl--line-height));\n  }\n\n  .text-xs {\n    font-size: var(--text-xs);\n    line-height: var(--tw-leading, var(--text-xs--line-height));\n  }\n\n  .text-\\[0\\.7rem\\] {\n    font-size: .7rem;\n  }\n\n  .text-\\[0\\.65rem\\] {\n    font-size: .65rem;\n  }\n\n  .text-\\[0\\.8125rem\\] {\n    font-size: .8125rem;\n  }\n\n  .font-medium {\n    --tw-font-weight: var(--font-weight-medium);\n    font-weight: var(--font-weight-medium);\n  }\n\n  .font-normal {\n    --tw-font-weight: var(--font-weight-normal);\n    font-weight: var(--font-weight-normal);\n  }\n\n  .font-semibold {\n    --tw-font-weight: var(--font-weight-semibold);\n    font-weight: var(--font-weight-semibold);\n  }\n\n  .tracking-tight {\n    --tw-tracking: var(--tracking-tight);\n    letter-spacing: var(--tracking-tight);\n  }\n\n  .tracking-wide {\n    --tw-tracking: var(--tracking-wide);\n    letter-spacing: var(--tracking-wide);\n  }\n\n  .tracking-wider {\n    --tw-tracking: var(--tracking-wider);\n    letter-spacing: var(--tracking-wider);\n  }\n\n  .uppercase {\n    text-transform: uppercase;\n  }\n\n  .italic {\n    font-style: italic;\n  }\n\n  .tabular-nums {\n    --tw-numeric-spacing: tabular-nums;\n    font-variant-numeric: var(--tw-ordinal, ) var(--tw-slashed-zero, ) var(--tw-numeric-figure, ) var(--tw-numeric-spacing, ) var(--tw-numeric-fraction, );\n  }\n\n  .no-underline {\n    text-decoration-line: none;\n  }\n\n  .opacity-70 {\n    opacity: .7;\n  }\n\n  .opacity-75 {\n    opacity: .75;\n  }\n\n  .transition-colors {\n    transition-property: color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to;\n    transition-timing-function: var(--tw-ease, var(--default-transition-timing-function));\n    transition-duration: var(--tw-duration, var(--default-transition-duration));\n  }\n\n  .transition-transform {\n    transition-property: transform, translate, scale, rotate;\n    transition-timing-function: var(--tw-ease, var(--default-transition-timing-function));\n    transition-duration: var(--tw-duration, var(--default-transition-duration));\n  }\n\n  .duration-150 {\n    --tw-duration: .15s;\n    transition-duration: .15s;\n  }\n\n  @media (hover: hover) {\n    .hover\\:underline:hover {\n      text-decoration-line: underline;\n    }\n  }\n\n  .disabled\\:opacity-50:disabled {\n    opacity: .5;\n  }\n\n  @media (min-width: 48rem) {\n    .md\\:grid-cols-4 {\n      grid-template-columns: repeat(4, minmax(0, 1fr));\n    }\n\n    .md\\:grid-cols-\\[260px_1fr\\] {\n      grid-template-columns: 260px 1fr;\n    }\n  }\n}\n\n@property --tw-space-y-reverse {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0;\n}\n\n@property --tw-border-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n\n@property --tw-font-weight {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-tracking {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ordinal {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-slashed-zero {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-figure {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-spacing {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-fraction {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-duration {\n  syntax: \"*\";\n  inherits: false\n}"));document.head.appendChild(elementStyle);}}catch(e){console.error('vite-plugin-css-injected-by-js', e);}

})();
import { jsx, jsxs, Fragment } from 'react/jsx-runtime';
import * as React from 'react';

const HOST_CLIENT_CTX_KEY = "__starterExtSdkHostClientContextV1";
globalThis[HOST_CLIENT_CTX_KEY] ?? (globalThis[HOST_CLIENT_CTX_KEY] = React.createContext(null));

const SLOT_CTX_KEY = "__starterExtSdkSlotContextV2";
const Context = globalThis[SLOT_CTX_KEY] ?? (globalThis[SLOT_CTX_KEY] = React.createContext(null));
function useSlotContext() {
  const ctx = React.useContext(Context);
  if (!ctx) {
    throw new Error(
      "useSlotContext() called outside <SlotContextProvider>. The host's federation runtime must wrap exposed components in SlotContextProvider."
    );
  }
  return ctx;
}
function useExtensionRoute() {
  return useSlotContext().route;
}

const DEFAULT_BLOCK_SHELL_MESSAGES = {
  loading: "Loading…",
  errorTitle: "Extension failed:"
};
function mergeBlockShellMessages(override) {
  return override ? { ...DEFAULT_BLOCK_SHELL_MESSAGES, ...override } : DEFAULT_BLOCK_SHELL_MESSAGES;
}
function BlockShell(props) {
  const slot = useSlotContext();
  const messages = React.useMemo(
    () => mergeBlockShellMessages(props.messages),
    [props.messages]
  );
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: props.className ? `starter-ext-block ${props.className}` : "starter-ext-block",
      "data-ext-id": slot.extensionId,
      "data-ext-slot": slot.slotId,
      children: /* @__PURE__ */ jsx(
        ExtensionErrorBoundary,
        {
          extensionId: slot.extensionId,
          fallback: props.errorFallback,
          errorTitle: messages.errorTitle,
          children: /* @__PURE__ */ jsx(
            React.Suspense,
            {
              fallback: props.loading ?? /* @__PURE__ */ jsx(DefaultLoading, { slotId: slot.slotId, label: messages.loading }),
              children: props.children
            }
          )
        }
      )
    }
  );
}
class ExtensionErrorBoundary extends React.Component {
  state = { error: null };
  static getDerivedStateFromError(error) {
    return { error };
  }
  componentDidCatch(error, info) {
    console.error(
      `[starter-ext] extension ${this.props.extensionId} crashed in render:`,
      error,
      info
    );
  }
  render() {
    if (this.state.error !== null) {
      const fb = this.props.fallback;
      if (fb) {
        return fb(this.state.error, this.props.extensionId);
      }
      return defaultErrorFallback(
        this.state.error,
        this.props.extensionId,
        this.props.errorTitle
      );
    }
    return this.props.children;
  }
}
function defaultErrorFallback(err, extensionId, title) {
  const msg = err instanceof Error ? err.message : String(err);
  return /* @__PURE__ */ jsxs("div", { role: "alert", className: "starter-ext-block__error", children: [
    /* @__PURE__ */ jsx("strong", { children: title }),
    " ",
    extensionId,
    /* @__PURE__ */ jsx("div", { children: msg })
  ] });
}
function DefaultLoading(props) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "aria-busy": "true",
      "aria-live": "polite",
      className: "starter-ext-block__loading",
      "data-slot": props.slotId,
      children: props.label
    }
  );
}

function useHostTheme() {
  const slot = useSlotContext();
  return React.useMemo(
    () => ({
      mode: slot.theme,
      tokens: slot.themeTokens,
      token(key) {
        const fromMap = slot.themeTokens?.[key];
        if (fromMap) return fromMap;
        if (typeof window === "undefined") return "";
        const styles = window.getComputedStyle(document.documentElement);
        return styles.getPropertyValue(`--${key}`).trim();
      }
    }),
    [slot.theme, slot.themeTokens]
  );
}

const HOST_BINDINGS_CTX_KEY = "__starterExtSdkHostBindingsContextV1";
const HostBindingsContext = globalThis[HOST_BINDINGS_CTX_KEY] ?? (globalThis[HOST_BINDINGS_CTX_KEY] = React.createContext(null));
function HostBindingsProvider(props) {
  return /* @__PURE__ */ jsx(HostBindingsContext.Provider, { value: props.bindings, children: props.children });
}

function registerExtensionContributions(handle, contributions) {
  const bindings = { extensionId: handle.id, singletons: handle.singletons };
  const wrapped = {};
  for (const [name, Component] of Object.entries(contributions.components)) {
    wrapped[name] = wrapWithBindings(name, Component, bindings);
  }
  handle.register({ components: wrapped });
}
function wrapWithBindings(displayName, Component, bindings) {
  const Wrapped = (props) => /* @__PURE__ */ jsx(HostBindingsProvider, { bindings, children: /* @__PURE__ */ jsx(Component, { ...props }) });
  Wrapped.displayName = `HostBindings(${bindings.extensionId}:${displayName})`;
  return Wrapped;
}

const EXTENSION_ID = "com.nubeio.rubixos";
function asNumber(v) {
  if (v === null || v === void 0 || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}

async function callTool(toolId, params) {
  const res = await fetch(`/api/v1/tools/${toolId}`, {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify(params ?? {})
  });
  const text = await res.text();
  let body = void 0;
  try {
    body = text ? JSON.parse(text) : void 0;
  } catch {
    body = text;
  }
  if (!res.ok) {
    const msg = body && typeof body === "object" && "error" in body ? String(body.error) : `HTTP ${res.status}`;
    throw new Error(msg);
  }
  return body;
}
async function fetchTemplate(template, params = {}) {
  const res = await callTool(
    `${EXTENSION_ID}.warehouse_query`,
    { template, params }
  );
  return res.rows;
}

function HistoryLineChart({
  rows,
  height = 220
}) {
  if (rows.length === 0) return null;
  const points = rows.map((r) => ({
    t: Date.parse(r.bucket),
    v: asNumber(r.avg_value)
  })).filter((p) => Number.isFinite(p.t) && p.v !== null);
  if (points.length === 0) return null;
  const tMin = points[0].t;
  const tMax = points[points.length - 1].t;
  const vMin = Math.min(...points.map((p) => p.v));
  const vMax = Math.max(...points.map((p) => p.v));
  const vSpan = vMax - vMin || 1;
  const tSpan = tMax - tMin || 1;
  const width = 720;
  const padL = 48;
  const padR = 12;
  const padT = 12;
  const padB = 28;
  const innerW = width - padL - padR;
  const innerH = height - padT - padB;
  const xOf = (t) => padL + (t - tMin) / tSpan * innerW;
  const yOf = (v) => padT + innerH - (v - vMin) / vSpan * innerH;
  const path = points.map((p, i) => `${i === 0 ? "M" : "L"}${xOf(p.t).toFixed(1)},${yOf(p.v).toFixed(1)}`).join(" ");
  const yTicks = [0, 1, 2, 3].map((i) => vMin + vSpan * i / 3);
  const xTicks = [0, 1, 2, 3, 4].map((i) => tMin + tSpan * i / 4);
  return /* @__PURE__ */ jsxs(
    "svg",
    {
      width: "100%",
      viewBox: `0 0 ${width} ${height}`,
      role: "img",
      "aria-label": "History (time-bucketed average)",
      className: "block",
      children: [
        yTicks.map((v, i) => {
          const y = yOf(v);
          return /* @__PURE__ */ jsxs("g", { opacity: 0.6, children: [
            /* @__PURE__ */ jsx(
              "line",
              {
                x1: padL,
                y1: y,
                x2: width - padR,
                y2: y,
                stroke: "currentColor",
                strokeWidth: 0.5,
                strokeDasharray: "2 3",
                opacity: 0.4
              }
            ),
            /* @__PURE__ */ jsx(
              "text",
              {
                x: padL - 6,
                y: y + 3,
                fontSize: 10,
                textAnchor: "end",
                fill: "currentColor",
                opacity: 0.8,
                children: v.toFixed(2)
              }
            )
          ] }, `y${i}`);
        }),
        xTicks.map((t, i) => {
          const x = xOf(t);
          const d = new Date(t);
          const label = `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
          return /* @__PURE__ */ jsx(
            "text",
            {
              x,
              y: height - 8,
              fontSize: 10,
              textAnchor: "middle",
              fill: "currentColor",
              opacity: 0.75,
              children: label
            },
            `x${i}`
          );
        }),
        /* @__PURE__ */ jsx("path", { d: path, stroke: "currentColor", strokeWidth: 1.4, fill: "none", className: "text-primary" }),
        points.map((p, i) => /* @__PURE__ */ jsx(
          "circle",
          {
            cx: xOf(p.t),
            cy: yOf(p.v),
            r: 1.8,
            className: "fill-primary"
          },
          i
        ))
      ]
    }
  );
}

function Main() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(MainRouter, {}) });
}
function MainRouter() {
  const route = useExtensionRoute();
  if (route === "hosts") return /* @__PURE__ */ jsx(HostsPage, {});
  if (route === "networks") return /* @__PURE__ */ jsx(NetworksPage, {});
  if (route === "devices") return /* @__PURE__ */ jsx(DevicesPage, {});
  if (route === "history" || route?.startsWith("history/")) return /* @__PURE__ */ jsx(HistoryPage, {});
  return /* @__PURE__ */ jsx(OverviewPage, {});
}
function OverviewPage() {
  const slot = useSlotContext();
  const theme = useHostTheme();
  const [detail, setDetail] = React.useState(null);
  const [summary, setSummary] = React.useState(null);
  const [hosts, setHosts] = React.useState([]);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(false);
  const [tick, setTick] = React.useState(0);
  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
        credentials: "same-origin",
        headers: { accept: "application/json" }
      }).then(async (r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return await r.json();
      }),
      fetchTemplate(`${EXTENSION_ID}.histories_summary`, {}),
      fetchTemplate(`${EXTENSION_ID}.hosts_overview`, { limit: 25 })
    ]).then(([d, s, h]) => {
      if (cancelled) return;
      setDetail(d);
      setSummary(s[0] ?? null);
      setHosts(h);
    }).catch((e) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    }).finally(() => {
      if (!cancelled) setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [tick]);
  const totalRows = summary ? Number(summary.sample_count) : null;
  return /* @__PURE__ */ jsxs(
    Page,
    {
      slot: slot.slotId,
      theme: theme.mode,
      header: /* @__PURE__ */ jsx(
        Header,
        {
          subtitle: "Nube-iO Rubix-OS BMS — devices · points · histories",
          version: detail?.manifest?.version,
          onRefresh: () => setTick((t) => t + 1),
          loading
        }
      ),
      error,
      children: [
        /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-2 md:grid-cols-4 gap-3", children: [
          /* @__PURE__ */ jsx(Kpi, { label: "Samples", value: fmtInt$1(totalRows) }),
          /* @__PURE__ */ jsx(Kpi, { label: "Points (with history)", value: fmtInt$1(summary?.point_count ?? null) }),
          /* @__PURE__ */ jsx(Kpi, { label: "Earliest", value: fmtTs(summary?.earliest ?? null) }),
          /* @__PURE__ */ jsx(Kpi, { label: "Latest", value: fmtTs(summary?.latest ?? null) })
        ] }),
        /* @__PURE__ */ jsx(Card, { title: "Hosts", description: `${EXTENSION_ID}.hosts_overview`, children: hosts.length === 0 ? /* @__PURE__ */ jsxs(Empty, { children: [
          "No hosts. Run ",
          /* @__PURE__ */ jsx("code", { children: "scripts/load-dump.sh" }),
          " to ingest a dump."
        ] }) : /* @__PURE__ */ jsx(Table, { headers: ["Host", "Networks", "Devices", "Points"], children: hosts.map((h) => /* @__PURE__ */ jsxs("tr", { className: "border-t border-border/60", children: [
          /* @__PURE__ */ jsxs(Td, { children: [
            /* @__PURE__ */ jsx(
              "a",
              {
                className: "text-primary hover:underline",
                href: `/extensions/${EXTENSION_ID}/networks?host=${encodeURIComponent(h.host_uuid)}`,
                children: h.host_name ?? /* @__PURE__ */ jsx(Mono, { children: h.host_uuid })
              }
            ),
            h.host_description ? /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground text-xs", children: [
              " · ",
              h.host_description
            ] }) : null
          ] }),
          /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.network_count) }),
          /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.device_count) }),
          /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.point_count) })
        ] }, h.host_uuid)) }) }),
        /* @__PURE__ */ jsx(Card, { title: "Contributions", description: "declared in block.yaml", children: /* @__PURE__ */ jsx(ContribGrid, { detail }) })
      ]
    }
  );
}
function HostsPage() {
  const rows = useTemplate(`${EXTENSION_ID}.hosts_overview`, { limit: 200 });
  return /* @__PURE__ */ jsx(SimplePage, { title: "Hosts", template: `${EXTENSION_ID}.hosts_overview`, rows, children: (data) => /* @__PURE__ */ jsx(Table, { headers: ["Host", "Networks", "Devices", "Points"], children: data.map((h) => /* @__PURE__ */ jsxs("tr", { className: "border-t border-border/60", children: [
    /* @__PURE__ */ jsx(Td, { children: h.host_name ?? /* @__PURE__ */ jsx(Mono, { children: h.host_uuid }) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.network_count) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.device_count) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(h.point_count) })
  ] }, h.host_uuid)) }) });
}
function NetworksPage() {
  const rows = useTemplate(`${EXTENSION_ID}.networks_overview`, { limit: 200 });
  return /* @__PURE__ */ jsx(SimplePage, { title: "Networks", template: `${EXTENSION_ID}.networks_overview`, rows, children: (data) => /* @__PURE__ */ jsx(Table, { headers: ["Network", "Host", "Devices", "Points"], children: data.map((n) => /* @__PURE__ */ jsxs("tr", { className: "border-t border-border/60", children: [
    /* @__PURE__ */ jsxs(Td, { children: [
      n.network_name ?? /* @__PURE__ */ jsx(Mono, { children: n.network_uuid }),
      n.network_description ? /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground text-xs", children: [
        " · ",
        n.network_description
      ] }) : null
    ] }),
    /* @__PURE__ */ jsx(Td, { children: n.host_name ?? /* @__PURE__ */ jsx(Mono, { children: n.host_uuid ?? "" }) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(n.device_count) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(n.point_count) })
  ] }, n.network_uuid)) }) });
}
function DevicesPage() {
  const rows = useTemplate(`${EXTENSION_ID}.devices_overview`, { limit: 200 });
  return /* @__PURE__ */ jsx(SimplePage, { title: "Devices", template: `${EXTENSION_ID}.devices_overview`, rows, children: (data) => /* @__PURE__ */ jsx(Table, { headers: ["Device", "Network", "Host", "Points"], children: data.map((d) => /* @__PURE__ */ jsxs("tr", { className: "border-t border-border/60", children: [
    /* @__PURE__ */ jsxs(Td, { children: [
      /* @__PURE__ */ jsx(
        "a",
        {
          className: "text-primary hover:underline",
          href: `/extensions/${EXTENSION_ID}/history?device=${encodeURIComponent(d.device_uuid)}`,
          children: d.device_name ?? /* @__PURE__ */ jsx(Mono, { children: d.device_uuid })
        }
      ),
      d.device_description ? /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground text-xs", children: [
        " · ",
        d.device_description
      ] }) : null
    ] }),
    /* @__PURE__ */ jsx(Td, { children: d.network_name ?? /* @__PURE__ */ jsx(Mono, { children: d.network_uuid ?? "" }) }),
    /* @__PURE__ */ jsx(Td, { children: d.host_name ?? /* @__PURE__ */ jsx(Mono, { children: d.host_uuid ?? "" }) }),
    /* @__PURE__ */ jsx(Td, { align: "right", children: fmtInt$1(d.point_count) })
  ] }, d.device_uuid)) }) });
}
const RANGES = [
  { label: "1h", hours: 1, bucket: "1 minute" },
  { label: "6h", hours: 6, bucket: "5 minutes" },
  { label: "24h", hours: 24, bucket: "15 minutes" },
  { label: "7d", hours: 168, bucket: "1 hour" },
  { label: "30d", hours: 720, bucket: "6 hours" },
  { label: "1y", hours: 8760, bucket: "1 day" }
];
function HistoryPage() {
  const slot = useSlotContext();
  const theme = useHostTheme();
  const params = React.useMemo(() => new URLSearchParams(window.location.search), []);
  const deviceFilter = params.get("device") ?? "";
  const [points, setPoints] = React.useState([]);
  const [pointsLoading, setPointsLoading] = React.useState(false);
  const [pointsError, setPointsError] = React.useState(null);
  const [selected, setSelected] = React.useState(null);
  const [rangeIdx, setRangeIdx] = React.useState(2);
  const [buckets, setBuckets] = React.useState([]);
  const [chartLoading, setChartLoading] = React.useState(false);
  const [chartError, setChartError] = React.useState(null);
  React.useEffect(() => {
    let cancelled = false;
    setPointsLoading(true);
    const tpl = deviceFilter ? { template: `${EXTENSION_ID}.points_by_device`, params: { device_uuid: deviceFilter, limit: 500 } } : { template: `${EXTENSION_ID}.points_list`, params: { limit: 200, offset: 0 } };
    fetchTemplate(tpl.template, tpl.params).then((rs) => {
      if (cancelled) return;
      setPoints(rs);
      if (rs.length > 0 && !selected) setSelected(rs[0].uuid);
    }).catch(
      (e) => !cancelled && setPointsError(e instanceof Error ? e.message : String(e))
    ).finally(() => !cancelled && setPointsLoading(false));
    return () => {
      cancelled = true;
    };
  }, [deviceFilter]);
  React.useEffect(() => {
    if (!selected) {
      setBuckets([]);
      return;
    }
    let cancelled = false;
    setChartLoading(true);
    setChartError(null);
    const r = RANGES[rangeIdx];
    const to = /* @__PURE__ */ new Date();
    const from = new Date(to.getTime() - r.hours * 36e5);
    fetchTemplate(`${EXTENSION_ID}.history_bucketed`, {
      point_uuid: selected,
      from: from.toISOString(),
      to: to.toISOString(),
      bucket: r.bucket
    }).then((rs) => !cancelled && setBuckets(rs)).catch(
      (e) => !cancelled && setChartError(e instanceof Error ? e.message : String(e))
    ).finally(() => !cancelled && setChartLoading(false));
    return () => {
      cancelled = true;
    };
  }, [selected, rangeIdx]);
  const selectedPoint = points.find((p) => p.uuid === selected) ?? null;
  return /* @__PURE__ */ jsx(
    Page,
    {
      slot: slot.slotId,
      theme: theme.mode,
      header: /* @__PURE__ */ jsx("div", { className: "flex items-center justify-between gap-4", children: /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsx("h3", { className: "text-lg font-semibold tracking-tight", children: "History" }),
        /* @__PURE__ */ jsxs("p", { className: "text-sm text-muted-foreground", children: [
          EXTENSION_ID,
          ".history_bucketed · Timescale ",
          `time_bucket()`,
          " aggregate",
          deviceFilter ? /* @__PURE__ */ jsxs(Fragment, { children: [
            " · filtered to device ",
            /* @__PURE__ */ jsx(Mono, { children: deviceFilter })
          ] }) : null
        ] })
      ] }) }),
      error: pointsError ?? chartError,
      children: /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-1 md:grid-cols-[260px_1fr] gap-4", children: [
        /* @__PURE__ */ jsx(Card, { title: "Points", description: pointsLoading ? "loading…" : `${points.length} points`, children: /* @__PURE__ */ jsxs("div", { className: "max-h-[60vh] overflow-y-auto -mx-1", children: [
          points.map((p) => /* @__PURE__ */ jsxs(
            "button",
            {
              type: "button",
              onClick: () => setSelected(p.uuid),
              className: "w-full text-left px-2 py-1.5 rounded text-sm hover:bg-accent " + (p.uuid === selected ? "bg-accent text-accent-foreground" : "text-foreground/85"),
              children: [
                /* @__PURE__ */ jsx("div", { className: "truncate", children: p.name ?? p.uuid }),
                p.device_name ? /* @__PURE__ */ jsx("div", { className: "truncate text-xs text-muted-foreground", children: p.device_name }) : null
              ]
            },
            p.uuid
          )),
          points.length === 0 && !pointsLoading ? /* @__PURE__ */ jsx(Empty, { children: "No points. Ingest the dump first." }) : null
        ] }) }),
        /* @__PURE__ */ jsxs(
          Card,
          {
            title: selectedPoint?.name ?? "Select a point",
            description: selectedPoint ? /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx(Mono, { children: selectedPoint.uuid }),
              selectedPoint.device_name ? /* @__PURE__ */ jsxs(Fragment, { children: [
                " · ",
                selectedPoint.device_name
              ] }) : null,
              selectedPoint.network_name ? /* @__PURE__ */ jsxs(Fragment, { children: [
                " · ",
                selectedPoint.network_name
              ] }) : null
            ] }) : "pick a point from the list to chart its history",
            children: [
              /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-1 mb-3", children: [
                RANGES.map((r, i) => /* @__PURE__ */ jsx(
                  "button",
                  {
                    type: "button",
                    onClick: () => setRangeIdx(i),
                    className: "px-2 py-1 text-xs rounded border transition-colors " + (i === rangeIdx ? "bg-primary text-primary-foreground border-primary" : "bg-transparent text-foreground border-border/60 hover:bg-accent"),
                    children: r.label
                  },
                  r.label
                )),
                /* @__PURE__ */ jsxs("span", { className: "ml-3 text-xs text-muted-foreground", children: [
                  "bucket = ",
                  RANGES[rangeIdx].bucket
                ] })
              ] }),
              chartLoading ? /* @__PURE__ */ jsx(Empty, { children: "loading…" }) : buckets.length === 0 ? /* @__PURE__ */ jsx(Empty, { children: "No samples in the selected range." }) : /* @__PURE__ */ jsxs("div", { className: "text-primary", children: [
                /* @__PURE__ */ jsx(HistoryLineChart, { rows: buckets }),
                /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-4 gap-2 mt-3 text-xs text-muted-foreground", children: [
                  /* @__PURE__ */ jsx(KpiSm, { label: "buckets", value: String(buckets.length) }),
                  /* @__PURE__ */ jsx(KpiSm, { label: "min", value: fmtNum(Math.min(...bucketAvgs(buckets))) }),
                  /* @__PURE__ */ jsx(KpiSm, { label: "max", value: fmtNum(Math.max(...bucketAvgs(buckets))) }),
                  /* @__PURE__ */ jsx(KpiSm, { label: "avg", value: fmtNum(mean(bucketAvgs(buckets))) })
                ] })
              ] })
            ]
          }
        )
      ] })
    }
  );
}
function bucketAvgs(rows) {
  return rows.map((r) => asNumber(r.avg_value)).filter((n) => n !== null);
}
function mean(xs) {
  if (xs.length === 0) return NaN;
  return xs.reduce((a, b) => a + b, 0) / xs.length;
}
function Page({
  slot,
  theme,
  header,
  error,
  children
}) {
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot,
      "data-ext-theme": theme,
      className: "flex flex-col gap-4 p-4",
      children: [
        header,
        error ? /* @__PURE__ */ jsx("div", { role: "alert", className: "rounded-md border border-destructive/40 bg-destructive/10 text-destructive px-3 py-2 text-sm", children: error }) : null,
        children
      ]
    }
  );
}
function Header({
  subtitle,
  version,
  onRefresh,
  loading
}) {
  return /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between gap-4", children: [
    /* @__PURE__ */ jsxs("div", { children: [
      /* @__PURE__ */ jsxs("h3", { className: "text-lg font-semibold tracking-tight", children: [
        "Rubix-OS",
        version ? /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground font-normal ml-2 text-sm", children: [
          "v",
          version
        ] }) : null
      ] }),
      /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: subtitle })
    ] }),
    /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        onClick: onRefresh,
        disabled: loading,
        className: "text-sm px-3 py-1 rounded border border-border/60 hover:bg-accent disabled:opacity-50",
        children: loading ? "loading…" : "refresh"
      }
    )
  ] });
}
function Card({
  title,
  description,
  children
}) {
  return /* @__PURE__ */ jsxs("section", { className: "rounded-lg border border-border/60 bg-card text-card-foreground", children: [
    /* @__PURE__ */ jsxs("header", { className: "px-3 py-2 border-b border-border/60", children: [
      /* @__PURE__ */ jsx("div", { className: "text-sm font-medium", children: title }),
      description ? /* @__PURE__ */ jsx("div", { className: "text-xs text-muted-foreground", children: description }) : null
    ] }),
    /* @__PURE__ */ jsx("div", { className: "p-3", children })
  ] });
}
function Kpi({ label, value }) {
  return /* @__PURE__ */ jsxs("div", { className: "rounded-lg border border-border/60 bg-card text-card-foreground p-3", children: [
    /* @__PURE__ */ jsx("div", { className: "text-xs uppercase tracking-wide text-muted-foreground", children: label }),
    /* @__PURE__ */ jsx("div", { className: "text-xl font-semibold tabular-nums", children: value })
  ] });
}
function KpiSm({ label, value }) {
  return /* @__PURE__ */ jsxs("div", { children: [
    /* @__PURE__ */ jsx("div", { className: "uppercase tracking-wide text-[0.65rem] opacity-75", children: label }),
    /* @__PURE__ */ jsx("div", { className: "text-sm font-medium tabular-nums text-foreground", children: value })
  ] });
}
function Table({
  headers,
  children
}) {
  return /* @__PURE__ */ jsx("div", { className: "overflow-x-auto", children: /* @__PURE__ */ jsxs("table", { className: "w-full text-sm", children: [
    /* @__PURE__ */ jsx("thead", { children: /* @__PURE__ */ jsx("tr", { className: "text-left text-xs text-muted-foreground", children: headers.map((h, i) => /* @__PURE__ */ jsx(
      "th",
      {
        className: "py-1.5 px-2 font-medium " + (i >= 1 && i === headers.length - 1 ? "text-right" : ""),
        children: h
      },
      h
    )) }) }),
    /* @__PURE__ */ jsx("tbody", { children })
  ] }) });
}
function Td({
  children,
  align
}) {
  return /* @__PURE__ */ jsx("td", { className: "py-1.5 px-2 " + (align === "right" ? "text-right tabular-nums" : ""), children });
}
function Empty({ children }) {
  return /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground italic", children });
}
function Mono({ children }) {
  return /* @__PURE__ */ jsx("code", { className: "text-xs font-mono", children });
}
function fmtInt$1(v) {
  if (v === null || v === void 0 || v === "") return "—";
  const n = typeof v === "number" ? v : Number(v);
  if (!Number.isFinite(n)) return "—";
  return n.toLocaleString();
}
function fmtNum(v) {
  if (v === null || v === void 0 || !Number.isFinite(v)) return "—";
  return v.toFixed(2);
}
function fmtTs(v) {
  if (!v) return "—";
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return v;
  return d.toLocaleString();
}
function SimplePage({
  title,
  template,
  rows,
  children
}) {
  const slot = useSlotContext();
  const theme = useHostTheme();
  return /* @__PURE__ */ jsx(
    Page,
    {
      slot: slot.slotId,
      theme: theme.mode,
      header: /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsx("h3", { className: "text-lg font-semibold tracking-tight", children: title }),
        /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: template })
      ] }),
      error: rows.error,
      children: /* @__PURE__ */ jsx(Card, { title, description: rows.loading ? "loading…" : `${rows.data.length} rows`, children: rows.data.length === 0 && !rows.loading ? /* @__PURE__ */ jsx(Empty, { children: "No rows. Ingest the dump first." }) : children(rows.data) })
    }
  );
}
function useTemplate(template, params) {
  const [state, setState] = React.useState({ data: [], loading: false, error: null });
  const key = JSON.stringify(params);
  React.useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));
    fetchTemplate(template, params).then((rs) => !cancelled && setState({ data: rs, loading: false, error: null })).catch(
      (e) => !cancelled && setState({ data: [], loading: false, error: e instanceof Error ? e.message : String(e) })
    );
    return () => {
      cancelled = true;
    };
  }, [template, key]);
  return state;
}
function ContribGrid({ detail }) {
  const c = detail?.manifest?.contributes ?? {};
  const rows = [
    ["tools", (c.tools ?? []).map((t) => t.id)],
    ["warehouse tables", (c.warehouse_tables ?? []).map((t) => t.name)],
    ["warehouse templates", (c.warehouse_templates ?? []).map((t) => t.name)],
    ["ui slots", (c.ui?.exposes ?? []).map((e) => e.slot)]
  ];
  return /* @__PURE__ */ jsx("div", { className: "flex flex-col gap-1.5", children: rows.map(([label, items]) => /* @__PURE__ */ jsxs("div", { className: "flex items-baseline gap-3 text-sm", children: [
    /* @__PURE__ */ jsx("span", { className: "text-muted-foreground shrink-0 w-44", children: label }),
    /* @__PURE__ */ jsx("span", { className: "flex flex-wrap gap-1", children: items.length === 0 ? /* @__PURE__ */ jsx("span", { className: "text-muted-foreground/50", children: "—" }) : items.map((id, i) => /* @__PURE__ */ jsx("code", { className: "rounded bg-muted px-1.5 py-0.5 text-xs font-mono", children: id }, id + i)) })
  ] }, label)) });
}

function isBranch(item) {
  return "children" in item;
}
const TREE = [
  { title: "Overview", href: `/extensions/${EXTENSION_ID}` },
  {
    title: "Topology",
    children: [
      { title: "Hosts", href: `/extensions/${EXTENSION_ID}/hosts` },
      { title: "Networks", href: `/extensions/${EXTENSION_ID}/networks` },
      { title: "Devices", href: `/extensions/${EXTENSION_ID}/devices` }
    ]
  },
  {
    title: "Data",
    children: [
      { title: "History (chart)", href: `/extensions/${EXTENSION_ID}/history` }
    ]
  }
];
function NavTree() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(NavTreeInner, {}) });
}
function NavTreeInner() {
  return /* @__PURE__ */ jsxs("nav", { "aria-label": "Rubix-OS", className: "mx-2 text-[0.8125rem] text-foreground", children: [
    /* @__PURE__ */ jsx("div", { className: "px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wider text-muted-foreground", children: "Rubix-OS" }),
    /* @__PURE__ */ jsx("ul", { className: "m-0 p-0 list-none", children: TREE.map(
      (item) => isBranch(item) ? /* @__PURE__ */ jsx(Branch, { branch: item }, item.title) : /* @__PURE__ */ jsx(TopLeaf, { leaf: item }, item.href)
    ) })
  ] });
}
function TopLeaf({ leaf }) {
  return /* @__PURE__ */ jsx("li", { children: /* @__PURE__ */ jsx(
    "a",
    {
      href: leaf.href,
      className: "block py-1 px-2 pl-4 no-underline text-foreground rounded-md hover:bg-accent hover:text-accent-foreground transition-colors",
      children: leaf.title
    }
  ) });
}
function Branch({ branch }) {
  const [open, setOpen] = React.useState(true);
  return /* @__PURE__ */ jsxs("li", { children: [
    /* @__PURE__ */ jsxs(
      "button",
      {
        type: "button",
        onClick: () => setOpen((v) => !v),
        "aria-expanded": open,
        className: "w-full flex items-center gap-1.5 py-1 px-2 bg-transparent border-0 text-foreground font-inherit cursor-pointer rounded-md text-left hover:bg-accent hover:text-accent-foreground transition-colors",
        children: [
          /* @__PURE__ */ jsx(Chevron, { open }),
          /* @__PURE__ */ jsx("span", { children: branch.title })
        ]
      }
    ),
    open ? /* @__PURE__ */ jsx("ul", { className: "m-0 pl-5 list-none border-l border-border ml-4", children: branch.children.map((leaf) => /* @__PURE__ */ jsx("li", { children: /* @__PURE__ */ jsx(
      "a",
      {
        href: leaf.href,
        className: "block py-1 px-2 no-underline text-foreground/85 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors",
        children: leaf.title
      }
    ) }, leaf.href)) }) : null
  ] });
}
function Chevron({ open }) {
  return /* @__PURE__ */ jsx(
    "svg",
    {
      width: "10",
      height: "10",
      viewBox: "0 0 10 10",
      "aria-hidden": "true",
      className: "shrink-0 opacity-70 transition-transform duration-150 " + (open ? "rotate-90" : ""),
      children: /* @__PURE__ */ jsx("path", { d: "M3 1.5 L7 5 L3 8.5", stroke: "currentColor", strokeWidth: "1.4", fill: "none", strokeLinecap: "round", strokeLinejoin: "round" })
    }
  );
}

function Sidebar() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(SidebarInner, {}) });
}
function SidebarInner() {
  const slot = useSlotContext();
  const [detail, setDetail] = React.useState(null);
  const [summary, setSummary] = React.useState(null);
  const [error, setError] = React.useState(null);
  React.useEffect(() => {
    let cancelled = false;
    Promise.all([
      fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
        credentials: "same-origin",
        headers: { accept: "application/json" }
      }).then(async (r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return await r.json();
      }),
      fetchTemplate(`${EXTENSION_ID}.histories_summary`, {}).catch(() => [])
    ]).then(([d, s]) => {
      if (cancelled) return;
      setDetail(d);
      setSummary(s[0] ?? null);
    }).catch((e) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    });
    return () => {
      cancelled = true;
    };
  }, []);
  const version = detail?.manifest?.version;
  const samples = summary ? Number(summary.sample_count) : null;
  const points = summary ? Number(summary.point_count) : null;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot.slotId,
      className: "mx-2 my-1 rounded-md border border-border/60 bg-card text-card-foreground px-3 py-2",
      children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-baseline justify-between gap-2", children: [
          /* @__PURE__ */ jsx("div", { className: "text-xs font-semibold", children: "Rubix-OS" }),
          version ? /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground text-[0.65rem]", children: [
            "v",
            version
          ] }) : null
        ] }),
        error ? /* @__PURE__ */ jsx("p", { role: "alert", className: "text-sm text-destructive mt-1", children: error }) : /* @__PURE__ */ jsxs("div", { className: "text-[0.65rem] text-muted-foreground mt-1 space-y-0.5", children: [
          /* @__PURE__ */ jsxs("div", { children: [
            "samples: ",
            /* @__PURE__ */ jsx("span", { className: "tabular-nums text-foreground", children: fmtInt(samples) })
          ] }),
          /* @__PURE__ */ jsxs("div", { children: [
            "points:  ",
            /* @__PURE__ */ jsx("span", { className: "tabular-nums text-foreground", children: fmtInt(points) })
          ] })
        ] }),
        /* @__PURE__ */ jsx(
          "a",
          {
            href: `/extensions/${EXTENSION_ID}`,
            className: "text-xs text-primary hover:underline mt-2 inline-block",
            children: "open dashboard →"
          }
        )
      ]
    }
  );
}
function fmtInt(v) {
  return v === null || !Number.isFinite(v) ? "—" : v.toLocaleString();
}

const factory = {
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, NavTree, Sidebar }
    });
  }
};

export { factory as default };
