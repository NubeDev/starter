// Importmap shim — extension bundles compiled with the modern JSX
// transform import `jsx`/`jsxs`/`Fragment` from `react/jsx-runtime`.
// React 17+ ships these from a dedicated entry; we re-export from
// the host's published runtime.
const J = /** @type {any} */ (globalThis).__rubixReactJsxRuntime;
if (!J) {
  throw new Error(
    "rubix jsx-runtime-shim: globalThis.__rubixReactJsxRuntime is unset."
  );
}
export const { jsx, jsxs, Fragment } = J;
