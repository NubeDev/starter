// Stream Builder: compose input + buffer + processors + output, validate, run.

import { useState } from "react";
import { fetchBuffers, fetchInputs, fetchOutputs, fetchProcessors } from "../../api/catalog";
import { runStream, validateStream, type RunResponse } from "../../api/streams";
import { assembleConfig, type Picked } from "../../builder/assemble";
import { ComponentForm } from "../../components/ComponentForm";
import { ResultTable } from "../../components/ResultTable";
import { useAsync } from "../../hooks/useAsync";
import { ProcessorChain } from "./ProcessorChain";

export function BuilderPage() {
  const inputs = useAsync(fetchInputs, []);
  const outputs = useAsync(fetchOutputs, []);
  const processors = useAsync(fetchProcessors, []);
  const buffers = useAsync(fetchBuffers, []);

  const [input, setInput] = useState<Picked | null>(null);
  const [buffer, setBuffer] = useState<Picked | null>(null);
  const [chain, setChain] = useState<Picked[]>([]);
  const [output, setOutput] = useState<Picked | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [result, setResult] = useState<RunResponse | null>(null);

  const ready = inputs.data && outputs.data && processors.data && buffers.data;
  if (!ready) return <div className="page">Loading catalogs…</div>;

  function config() {
    if (!input || !output) throw new Error("Pick an input and an output first.");
    return assembleConfig({ input, buffer, processors: chain, output });
  }

  async function validate() {
    setResult(null);
    try {
      const r = await validateStream(config());
      setStatus(r.ok ? "✓ valid config" : `✗ ${r.error}`);
    } catch (e) {
      setStatus(`✗ ${(e as Error).message}`);
    }
  }

  async function run() {
    setStatus("running…");
    setResult(null);
    try {
      const r = await runStream(config());
      setResult(r);
      setStatus(r.ok ? `✓ ${r.row_count} rows${r.cancelled ? " (stopped at timeout)" : ""}` : `✗ ${r.error}`);
    } catch (e) {
      setStatus(`✗ ${(e as Error).message}`);
    }
  }

  return (
    <div className="page">
      <header className="page-head">
        <h1>Stream Builder</h1>
        <p>Compose a pipeline and run it on the embedded ArkFlow engine. Output is captured in memory.</p>
      </header>

      <div className="builder-grid">
        <ComponentForm title="Input" kinds={inputs.data!} picked={input} onChange={setInput} />
        <ComponentForm title="Buffer (optional)" kinds={buffers.data!} picked={buffer} optional onChange={setBuffer} />
        <ProcessorChain kinds={processors.data!} chain={chain} onChange={setChain} />
        <ComponentForm title="Output" kinds={outputs.data!} picked={output} onChange={setOutput} />
      </div>

      <div className="actions">
        <button className="ghost" onClick={validate}>
          Validate
        </button>
        <button className="primary" onClick={run}>
          Run
        </button>
        {status && <span className="status">{status}</span>}
      </div>

      {result?.ok && <ResultTable rows={result.rows} />}
    </div>
  );
}
