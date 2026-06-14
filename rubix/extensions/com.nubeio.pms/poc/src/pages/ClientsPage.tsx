import { useState } from "react";
import { Plus, Trash2, MapPin, Building2 } from "lucide-react";
import { useStore } from "@/store/store";
import { Card, Field, TextInput, SectionTitle, Empty, Chip } from "@/components/ui";

export function ClientsPage() {
  const { state, dispatch } = useStore();
  const [selected, setSelected] = useState<string | null>(state.clients[0]?.id ?? null);
  const [newClient, setNewClient] = useState("");
  const [site, setSite] = useState({ name: "", address: "" });

  const sitesFor = (cid: string) => state.sites.filter((s) => s.clientId === cid);

  return (
    <>
      <SectionTitle title="Clients & Sites" sub="Admin manages the client organisations and their sites." />

      <div className="grid grid-cols-[1fr_1.4fr] gap-6">
        {/* Clients column */}
        <div>
          <Card className="mb-4">
            <span className="label">Add client</span>
            <div className="flex gap-2">
              <TextInput
                placeholder="Client name"
                value={newClient}
                onChange={(e) => setNewClient(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newClient.trim()) {
                    dispatch({ type: "ADD_CLIENT", client: { name: newClient.trim() } });
                    setNewClient("");
                  }
                }}
              />
              <button
                className="btn btn-primary"
                onClick={() => {
                  if (newClient.trim()) {
                    dispatch({ type: "ADD_CLIENT", client: { name: newClient.trim() } });
                    setNewClient("");
                  }
                }}
              >
                <Plus size={15} />
              </button>
            </div>
          </Card>

          <div className="space-y-2">
            {state.clients.length === 0 && <Empty>No clients yet.</Empty>}
            {state.clients.map((c) => (
              <Card
                key={c.id}
                className={`cursor-pointer ${selected === c.id ? "border-accent" : ""}`}
              >
                <div className="flex items-center justify-between" onClick={() => setSelected(c.id)}>
                  <div className="flex items-center gap-2">
                    <Building2 size={16} className="text-accent" />
                    <div>
                      <div className="font-medium">{c.name}</div>
                      <div className="text-xs text-muted">{sitesFor(c.id).length} site(s)</div>
                    </div>
                  </div>
                  <button
                    className="btn btn-ghost btn-danger"
                    onClick={(e) => {
                      e.stopPropagation();
                      if (confirm(`Delete ${c.name} and its sites/projects?`))
                        dispatch({ type: "DELETE_CLIENT", id: c.id });
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </Card>
            ))}
          </div>
        </div>

        {/* Sites column */}
        <div>
          {selected ? (
            <>
              <Card className="mb-4">
                <span className="label">Add site to {state.clients.find((c) => c.id === selected)?.name}</span>
                <div className="grid grid-cols-2 gap-3">
                  <Field label="Site name">
                    <TextInput value={site.name} onChange={(e) => setSite({ ...site, name: e.target.value })} />
                  </Field>
                  <Field label="Address">
                    <TextInput value={site.address} onChange={(e) => setSite({ ...site, address: e.target.value })} />
                  </Field>
                </div>
                <button
                  className="btn btn-primary mt-3"
                  onClick={() => {
                    if (site.name.trim()) {
                      dispatch({
                        type: "ADD_SITE",
                        site: { clientId: selected, name: site.name.trim(), address: site.address.trim() },
                      });
                      setSite({ name: "", address: "" });
                    }
                  }}
                >
                  <Plus size={15} /> Add site
                </button>
              </Card>

              <div className="space-y-2">
                {sitesFor(selected).length === 0 && <Empty>No sites for this client yet.</Empty>}
                {sitesFor(selected).map((s) => (
                  <Card key={s.id} className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <MapPin size={16} className="text-accent-2" />
                      <div>
                        <div className="font-medium">{s.name}</div>
                        <div className="text-xs text-muted">{s.address || "no address"}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Chip>{s.id}</Chip>
                      <button
                        className="btn btn-ghost btn-danger"
                        onClick={() => dispatch({ type: "DELETE_SITE", id: s.id })}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </Card>
                ))}
              </div>
            </>
          ) : (
            <Empty>Select a client to manage its sites.</Empty>
          )}
        </div>
      </div>
    </>
  );
}
