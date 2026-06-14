import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Plus, Trash2, FolderKanban, ArrowRight } from "lucide-react";
import { useStore } from "@/store/store";
import { Card, Field, Select, TextInput, SectionTitle, Empty, Chip } from "@/components/ui";
import type { Project } from "@/types";

export function ProjectsPage() {
  const { state, dispatch, newId } = useStore();
  const nav = useNavigate();
  const [name, setName] = useState("");
  const [clientId, setClientId] = useState(state.clients[0]?.id ?? "");
  const sitesFor = state.sites.filter((s) => s.clientId === clientId);
  const [siteId, setSiteId] = useState(sitesFor[0]?.id ?? "");

  const create = () => {
    if (!name.trim() || !clientId || !siteId) {
      alert("Pick a client, a site, and a project name.");
      return;
    }
    const project: Project = {
      id: newId("proj"),
      clientId,
      siteId,
      name: name.trim(),
      createdAt: new Date().toISOString().slice(0, 16).replace("T", " "),
      gateways: [],
    };
    dispatch({ type: "UPSERT_PROJECT", project });
    nav(`/projects/${project.id}`);
  };

  return (
    <>
      <SectionTitle title="Projects" sub="Clients build a project against a site by composing gateways and devices." />

      <Card className="mb-6">
        <span className="label">New project</span>
        <div className="grid grid-cols-[1fr_1fr_1fr_auto] gap-3 items-end">
          <Field label="Client">
            <Select
              value={clientId}
              onChange={(e) => {
                setClientId(e.target.value);
                const first = state.sites.find((s) => s.clientId === e.target.value);
                setSiteId(first?.id ?? "");
              }}
            >
              {state.clients.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Site">
            <Select value={siteId} onChange={(e) => setSiteId(e.target.value)}>
              {sitesFor.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Project name">
            <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Tower L1-L5 BMS" />
          </Field>
          <button className="btn btn-primary" onClick={create}>
            <Plus size={15} /> Create
          </button>
        </div>
      </Card>

      <div className="space-y-2">
        {state.projects.length === 0 && <Empty>No projects yet. Create one above.</Empty>}
        {state.projects.map((p) => {
          const site = state.sites.find((s) => s.id === p.siteId);
          const client = state.clients.find((c) => c.id === p.clientId);
          const ndev = p.gateways.reduce(
            (n, g) => n + g.buses.reduce((m, b) => m + b.devices.length, 0),
            0,
          );
          return (
            <Card key={p.id} className="flex items-center justify-between">
              <Link to={`/projects/${p.id}`} className="flex items-center gap-3 flex-1">
                <FolderKanban size={18} className="text-accent" />
                <div>
                  <div className="font-medium">{p.name}</div>
                  <div className="text-xs text-muted">
                    {client?.name} · {site?.name} · {p.createdAt}
                  </div>
                </div>
              </Link>
              <div className="flex items-center gap-2">
                <Chip>{p.gateways.length} gw</Chip>
                <Chip>{ndev} dev</Chip>
                <Link to={`/projects/${p.id}`} className="btn btn-ghost">
                  Open <ArrowRight size={14} />
                </Link>
                <button
                  className="btn btn-ghost btn-danger"
                  onClick={() => {
                    if (confirm(`Delete project ${p.name}?`)) dispatch({ type: "DELETE_PROJECT", id: p.id });
                  }}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </Card>
          );
        })}
      </div>
    </>
  );
}
