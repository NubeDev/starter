(function () {
	'use strict';

	try{if(typeof document != 'undefined'){var elementStyle = document.createElement('style');elementStyle.appendChild(document.createTextNode("/*! tailwindcss v4.3.0 | MIT License | https://tailwindcss.com */\n@layer properties {\n  @supports (((-webkit-hyphens: none)) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color: rgb(from red r g b)))) {\n    *, :before, :after, ::backdrop {\n      --tw-border-style: solid;\n      --tw-font-weight: initial;\n      --tw-tracking: initial;\n    }\n  }\n}\n\n@media no-preflight {\n  @layer utilities {\n    @layer theme {\n      :root, :host {\n        --spacing: .25rem;\n        --text-xs: .75rem;\n        --text-xs--line-height: calc(1 / .75);\n        --text-sm: .875rem;\n        --text-sm--line-height: calc(1.25 / .875);\n        --text-lg: 1.125rem;\n        --text-lg--line-height: calc(1.75 / 1.125);\n        --font-weight-medium: 500;\n        --font-weight-semibold: 600;\n        --tracking-wider: .05em;\n        --default-transition-duration: .15s;\n        --default-transition-timing-function: cubic-bezier(.4, 0, .2, 1);\n      }\n    }\n\n    @layer base {\n      *, :after, :before, ::backdrop {\n        box-sizing: border-box;\n        border: 0 solid;\n        margin: 0;\n        padding: 0;\n      }\n\n      ::file-selector-button {\n        box-sizing: border-box;\n        border: 0 solid;\n        margin: 0;\n        padding: 0;\n      }\n\n      html, :host {\n        -webkit-text-size-adjust: 100%;\n        tab-size: 4;\n        line-height: 1.5;\n        font-family: var(--default-font-family, ui-sans-serif, system-ui, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Segoe UI Symbol\", \"Noto Color Emoji\");\n        font-feature-settings: var(--default-font-feature-settings, normal);\n        font-variation-settings: var(--default-font-variation-settings, normal);\n        -webkit-tap-highlight-color: transparent;\n      }\n\n      hr {\n        height: 0;\n        color: inherit;\n        border-top-width: 1px;\n      }\n\n      abbr:where([title]) {\n        -webkit-text-decoration: underline dotted;\n        text-decoration: underline dotted;\n      }\n\n      h1, h2, h3, h4, h5, h6 {\n        font-size: inherit;\n        font-weight: inherit;\n      }\n\n      a {\n        color: inherit;\n        -webkit-text-decoration: inherit;\n        -webkit-text-decoration: inherit;\n        -webkit-text-decoration: inherit;\n        text-decoration: inherit;\n      }\n\n      b, strong {\n        font-weight: bolder;\n      }\n\n      code, kbd, samp, pre {\n        font-family: var(--default-mono-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace);\n        font-feature-settings: var(--default-mono-font-feature-settings, normal);\n        font-variation-settings: var(--default-mono-font-variation-settings, normal);\n        font-size: 1em;\n      }\n\n      small {\n        font-size: 80%;\n      }\n\n      sub, sup {\n        vertical-align: baseline;\n        font-size: 75%;\n        line-height: 0;\n        position: relative;\n      }\n\n      sub {\n        bottom: -.25em;\n      }\n\n      sup {\n        top: -.5em;\n      }\n\n      table {\n        text-indent: 0;\n        border-color: inherit;\n        border-collapse: collapse;\n      }\n\n      :-moz-focusring {\n        outline: auto;\n      }\n\n      progress {\n        vertical-align: baseline;\n      }\n\n      summary {\n        display: list-item;\n      }\n\n      ol, ul, menu {\n        list-style: none;\n      }\n\n      img, svg, video, canvas, audio, iframe, embed, object {\n        vertical-align: middle;\n        display: block;\n      }\n\n      img, video {\n        max-width: 100%;\n        height: auto;\n      }\n\n      button, input, select, optgroup, textarea {\n        font: inherit;\n        font-feature-settings: inherit;\n        font-variation-settings: inherit;\n        letter-spacing: inherit;\n        color: inherit;\n        opacity: 1;\n        background-color: #0000;\n        border-radius: 0;\n      }\n\n      ::file-selector-button {\n        font: inherit;\n        font-feature-settings: inherit;\n        font-variation-settings: inherit;\n        letter-spacing: inherit;\n        color: inherit;\n        opacity: 1;\n        background-color: #0000;\n        border-radius: 0;\n      }\n\n      :where(select:is([multiple], [size])) optgroup {\n        font-weight: bolder;\n      }\n\n      :where(select:is([multiple], [size])) optgroup option {\n        padding-inline-start: 20px;\n      }\n\n      ::file-selector-button {\n        margin-inline-end: 4px;\n      }\n\n      ::placeholder {\n        opacity: 1;\n      }\n\n      @supports (not ((-webkit-appearance: -apple-pay-button))) or (contain-intrinsic-size: 1px) {\n        ::placeholder {\n          color: currentColor;\n        }\n\n        @supports (color: color-mix(in lab, red, red)) {\n          ::placeholder {\n            color: color-mix(in oklab, currentcolor 50%, transparent);\n          }\n        }\n      }\n\n      textarea {\n        resize: vertical;\n      }\n\n      ::-webkit-search-decoration {\n        -webkit-appearance: none;\n      }\n\n      ::-webkit-date-and-time-value {\n        min-height: 1lh;\n        text-align: inherit;\n      }\n\n      ::-webkit-datetime-edit {\n        display: inline-flex;\n      }\n\n      ::-webkit-datetime-edit-fields-wrapper {\n        padding: 0;\n      }\n\n      ::-webkit-datetime-edit {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-year-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-month-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-day-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-hour-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-minute-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-second-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-millisecond-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-datetime-edit-meridiem-field {\n        padding-block: 0;\n      }\n\n      ::-webkit-calendar-picker-indicator {\n        line-height: 1;\n      }\n\n      :-moz-ui-invalid {\n        box-shadow: none;\n      }\n\n      button, input:where([type=\"button\"], [type=\"reset\"], [type=\"submit\"]) {\n        appearance: button;\n      }\n\n      ::file-selector-button {\n        appearance: button;\n      }\n\n      ::-webkit-inner-spin-button {\n        height: auto;\n      }\n\n      ::-webkit-outer-spin-button {\n        height: auto;\n      }\n\n      [hidden]:where(:not([hidden=\"until-found\"])) {\n        display: none !important;\n      }\n    }\n\n    @layer components;\n\n    @layer utilities {\n      .visible {\n        visibility: visible;\n      }\n\n      .m-0 {\n        margin: calc(var(--spacing) * 0);\n      }\n\n      .mx-2 {\n        margin-inline: calc(var(--spacing) * 2);\n      }\n\n      .my-1 {\n        margin-block: calc(var(--spacing) * 1);\n      }\n\n      .mt-1 {\n        margin-top: calc(var(--spacing) * 1);\n      }\n\n      .block {\n        display: block;\n      }\n\n      .inline-block {\n        display: inline-block;\n      }\n\n      .table {\n        display: table;\n      }\n\n      .list-none {\n        list-style-type: none;\n      }\n\n      .border {\n        border-style: var(--tw-border-style);\n        border-width: 1px;\n      }\n\n      .p-0 {\n        padding: calc(var(--spacing) * 0);\n      }\n\n      .p-4 {\n        padding: calc(var(--spacing) * 4);\n      }\n\n      .px-2 {\n        padding-inline: calc(var(--spacing) * 2);\n      }\n\n      .px-3 {\n        padding-inline: calc(var(--spacing) * 3);\n      }\n\n      .py-1 {\n        padding-block: calc(var(--spacing) * 1);\n      }\n\n      .py-2 {\n        padding-block: calc(var(--spacing) * 2);\n      }\n\n      .pl-4 {\n        padding-left: calc(var(--spacing) * 4);\n      }\n\n      .text-lg {\n        font-size: var(--text-lg);\n        line-height: var(--tw-leading, var(--text-lg--line-height));\n      }\n\n      .text-sm {\n        font-size: var(--text-sm);\n        line-height: var(--tw-leading, var(--text-sm--line-height));\n      }\n\n      .text-xs {\n        font-size: var(--text-xs);\n        line-height: var(--tw-leading, var(--text-xs--line-height));\n      }\n\n      .text-\\[0\\.7rem\\] {\n        font-size: .7rem;\n      }\n\n      .text-\\[0\\.8125rem\\] {\n        font-size: .8125rem;\n      }\n\n      .font-medium {\n        --tw-font-weight: var(--font-weight-medium);\n        font-weight: var(--font-weight-medium);\n      }\n\n      .font-semibold {\n        --tw-font-weight: var(--font-weight-semibold);\n        font-weight: var(--font-weight-semibold);\n      }\n\n      .tracking-wider {\n        --tw-tracking: var(--tracking-wider);\n        letter-spacing: var(--tracking-wider);\n      }\n\n      .uppercase {\n        text-transform: uppercase;\n      }\n\n      .no-underline {\n        text-decoration-line: none;\n      }\n\n      .transition-colors {\n        transition-property: color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to;\n        transition-timing-function: var(--tw-ease, var(--default-transition-timing-function));\n        transition-duration: var(--tw-duration, var(--default-transition-duration));\n      }\n\n      @media (hover: hover) {\n        .hover\\:underline:hover {\n          text-decoration-line: underline;\n        }\n      }\n    }\n  }\n}\n\n@property --tw-border-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n\n@property --tw-font-weight {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-tracking {\n  syntax: \"*\";\n  inherits: false\n}"));document.head.appendChild(elementStyle);}}catch(e){console.error('vite-plugin-css-injected-by-js', e);}

})();
import { jsx, jsxs } from 'react/jsx-runtime';
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

function Main() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsxs("div", { className: "p-4", children: [
    /* @__PURE__ */ jsx("h3", { className: "text-lg font-semibold", children: "Rubix Geo" }),
    /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: "Map view — TODO" })
  ] }) });
}

const EXTENSION_ID = "com.rubix.geo";

const TREE = [
  { title: "Map", href: `/extensions/${EXTENSION_ID}` },
  { title: "Layers", href: `/extensions/${EXTENSION_ID}/layers` },
  { title: "Pins", href: `/extensions/${EXTENSION_ID}/pins` }
];
function NavTree() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsxs("nav", { className: "mx-2 text-[0.8125rem] text-foreground", children: [
    /* @__PURE__ */ jsx("div", { className: "px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wider text-muted-foreground", children: "Geo" }),
    /* @__PURE__ */ jsx("ul", { className: "m-0 p-0 list-none", children: TREE.map((item) => /* @__PURE__ */ jsx("li", { children: /* @__PURE__ */ jsx(
      "a",
      {
        href: item.href,
        className: "block py-1 px-2 pl-4 no-underline text-foreground rounded-md hover:bg-accent hover:text-accent-foreground transition-colors",
        children: item.title
      }
    ) }, item.href)) })
  ] }) });
}

function Sidebar() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsxs("div", { className: "mx-2 my-1 py-2 px-3 rounded-md border border-border", children: [
    /* @__PURE__ */ jsx("div", { className: "text-xs font-medium", children: "Rubix Geo" }),
    /* @__PURE__ */ jsx(
      "a",
      {
        href: `/extensions/${EXTENSION_ID}`,
        className: "text-xs text-primary hover:underline mt-1 inline-block",
        children: "open map →"
      }
    )
  ] }) });
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
