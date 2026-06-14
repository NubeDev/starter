// SQL playground: run DataFusion SQL over an inline JSON dataset.

import { useState } from "react";
import { runSql, type SqlResponse } from "../api/sql";
import { ResultTable } from "../components/ResultTable";

const SAMPLE_ROWS = `[
  { "sensor": "temp_1", "value": 22.5 },
  { "sensor": "temp_2", "value": 9.1 },
  { "sensor": "temp_3", "value": 30.0 }
]`;

export function SqlPage() {
  const [query, setQuery] = useState("SELECT sensor, value FROM flow WHERE value >= 10");
  const [rowsText, setRowsText] = useState(SAMPLE_ROWS);
  const [result, setResult] = useState<SqlResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run() {
    setError(null);
    setBusy(true);
    try {
      const rows = JSON.parse(rowsText) as unknown[];
      setResult(await runSql(query, rows));
    } catch (e) {
      setError((e as Error).message);
      setResult(null);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <header className="page-head">
        <h1>SQL Playground</h1>
        <p>Runs through a real ArkFlow stream: memory input → json_to_arrow → sql → collector.</p>
      </header>

      <div className="editor-grid">
        <label className="field">
          <span className="field-label">Dataset (JSON rows)</span>
          <textarea rows={10} value={rowsText} onChange={(e) => setRowsText(e.target.value)} />
        </label>
        <label className="field">
          <span className="field-label">Query — the dataset is table `flow`</span>
          <textarea rows={10} value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
      </div>

      <button className="primary" onClick={run} disabled={busy}>
        {busy ? "Running…" : "Run query"}
      </button>

      {error && <p className="error">{error}</p>}
      {result?.error && <p className="error">{result.error}</p>}
      {result?.ok && (
        <>
          <p className="muted">{result.row_count} rows</p>
          <ResultTable rows={result.rows} />
        </>
      )}
    </div>
  );
}
