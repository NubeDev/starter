// Plugin manager: every registered component, grouped by category.

import { fetchPlugins } from "../api/catalog";
import { useAsync } from "../hooks/useAsync";

export function PluginsPage() {
  const { data, error, loading } = useAsync(fetchPlugins, []);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Plugins</h1>
        <p>Components registered in the engine. Custom plugins are registered by this POC at startup.</p>
      </header>
      {loading && <p className="muted">Loading…</p>}
      {error && <p className="error">{error}</p>}
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Type</th>
              <th>Category</th>
              <th>Source</th>
            </tr>
          </thead>
          <tbody>
            {data?.map((p) => (
              <tr key={`${p.category}:${p.type}`}>
                <td>
                  <code>{p.type}</code>
                </td>
                <td>{p.category}</td>
                <td>
                  <span className={p.source === "custom" ? "badge custom" : "badge"}>{p.source}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
