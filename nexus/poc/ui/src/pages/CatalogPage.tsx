// Read-only reference for one component category (inputs/outputs/processors/buffers).

import type { ComponentKind } from "../api/catalog";
import { useAsync } from "../hooks/useAsync";

interface Props {
  title: string;
  blurb: string;
  load: () => Promise<ComponentKind[]>;
}

export function CatalogPage({ title, blurb, load }: Props) {
  const { data, error, loading } = useAsync(load, [title]);

  return (
    <div className="page">
      <header className="page-head">
        <h1>{title}</h1>
        <p>{blurb}</p>
      </header>
      {loading && <p className="muted">Loading…</p>}
      {error && <p className="error">{error}</p>}
      <div className="grid">
        {data?.map((kind) => (
          <article key={kind.type} className="catalog-card">
            <header>
              <h3>{kind.label}</h3>
              <code>{kind.type}</code>
            </header>
            <p>{kind.summary}</p>
            {kind.fields.length > 0 && (
              <ul className="fields">
                {kind.fields.map((f) => (
                  <li key={f.name}>
                    <span className="fname">{f.name}</span>
                    <span className="ftag">{f.kind}</span>
                    {f.required && <span className="ftag req">required</span>}
                  </li>
                ))}
              </ul>
            )}
          </article>
        ))}
      </div>
    </div>
  );
}
