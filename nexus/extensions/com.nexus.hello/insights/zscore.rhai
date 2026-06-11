// com.nexus.hello.zscore — the demo extension-contributed insight (RW-07).
//
// Proves the `contributes.insights[]` path end to end: this script is
// compile-checked at boot, materialised into the global extension-insight
// registry, and applied by `POST /query { insight: { insight_name:
// "com.nexus.hello.zscore" } }`.
//
// It adds a z-score column for a chosen numeric column and flags rows whose
// score exceeds a threshold. `params.column` names the column (default "value");
// `params.threshold` is the z cutoff (default 3.0). Both primitives are
// row-count-preserving, so the result can never exceed the query caps.
let column = if params.column == () { "value" } else { params.column };
let threshold = if params.threshold == () { 3.0 } else { params.threshold };

df.zscore(column).anomalies(column, threshold)
