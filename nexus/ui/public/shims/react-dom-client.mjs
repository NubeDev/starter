// Importmap shim for `react-dom/client` — used by extensions that
// might call `createRoot`. We re-export from the host's react-dom.
const RDC = /** @type {any} */ (globalThis).__rubixReactDomClient;
if (!RDC) {
  throw new Error(
    "rubix react-dom-client-shim: globalThis.__rubixReactDomClient is unset."
  );
}
export const { createRoot, hydrateRoot } = RDC;
