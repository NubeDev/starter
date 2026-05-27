// `ui/main.tsx` — developer-facing source for the UI panel served
// by `com.rubix.example`. All data flows through the host's tool
// API; no bundled samples.

import * as React from "react";
import "./app.css";

import {
  BlockShell,
  useExtensionRoute,
  useHostTheme,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID } from "./types";
import type {
  CountryBucket,
  CustomerRow,
  ExtensionDetail,
  ProductRow,
  WarehouseQueryResponse,
} from "./types";
import { evaluateCustomerQuality } from "./quality";
import { ContribRow, CountryBarChart } from "./components";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { Badge } from "./components/ui/badge";
import { Input } from "./components/ui/input";
import { Label } from "./components/ui/label";
import { Alert, AlertDescription } from "./components/ui/alert";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./components/ui/table";
import { cn } from "./lib/utils";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainRouter />
    </BlockShell>
  );
}

function MainRouter(): React.ReactElement {
  const route = useExtensionRoute();
  if (route === "products/catalog" || route === "products/crud") {
    return <ProductsCrudPanel />;
  }
  return <MainInner />;
}

/* ----------------------------- helpers ------------------------------ */

async function callTool<T>(toolId: string, params: unknown): Promise<T> {
  const res = await fetch(`/api/v1/tools/${toolId}`, {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify(params ?? {}),
  });
  const text = await res.text();
  let body: unknown = undefined;
  try {
    body = text ? JSON.parse(text) : undefined;
  } catch {
    body = text;
  }
  if (!res.ok) {
    const msg =
      body && typeof body === "object" && "error" in body
        ? String((body as { error: unknown }).error)
        : `HTTP ${res.status}`;
    throw new Error(msg);
  }
  return body as T;
}

async function fetchTemplate<R>(
  template: string,
  params: Record<string, unknown> = {},
): Promise<ReadonlyArray<R>> {
  const res = await callTool<WarehouseQueryResponse<R>>(
    `${EXTENSION_ID}.warehouse_query`,
    { template, params },
  );
  return res.rows;
}

/* ----------------------------- overview ----------------------------- */

function MainInner(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();
  const [detail, setDetail] = React.useState<ExtensionDetail | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [tick, setTick] = React.useState(0);

  const [countryBuckets, setCountryBuckets] = React.useState<ReadonlyArray<CountryBucket>>([]);
  const [lowStock, setLowStock] = React.useState<ReadonlyArray<ProductRow>>([]);
  const [customers, setCustomers] = React.useState<ReadonlyArray<CustomerRow>>([]);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
        credentials: "same-origin",
        headers: { accept: "application/json" },
      }).then(async (res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return (await res.json()) as ExtensionDetail;
      }),
      fetchTemplate<CountryBucket>(`${EXTENSION_ID}.customers_by_country`, { limit: 10 }),
      fetchTemplate<ProductRow>(`${EXTENSION_ID}.products_low_stock`, { threshold: 10, limit: 50 }),
      fetchTemplate<CustomerRow>(`${EXTENSION_ID}.customers_sample`, { limit: 50 }),
    ])
      .then(([d, buckets, low, sample]) => {
        if (cancelled) return;
        setDetail(d);
        setCountryBuckets(buckets);
        setLowStock(low);
        setCustomers(sample);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [tick]);

  const c = detail?.manifest?.contributes ?? {};
  const tools = (c.tools ?? []).map((t) => t.id);
  const warehouseTables = (c.warehouse_tables ?? []).map((t) => t.name);
  const warehouseTemplates = (c.warehouse_templates ?? []).map((t) => t.name);
  const anomalyRules = (c.anomaly_rules ?? []).map((r) => r.id);
  const exposes = (c.ui?.exposes ?? []).map((e) => e.slot);

  const flagged = React.useMemo(
    () =>
      customers
        .map((r) => ({ r, q: evaluateCustomerQuality(r) }))
        .filter((x) => x.q.outcome !== "ok"),
    [customers],
  );

  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      data-ext-theme={theme.mode}
      className="flex flex-col gap-4 p-4"
    >
      {/* Header */}
      <div className="flex items-center justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold tracking-tight">
            {EXTENSION_ID}
            {detail?.manifest?.version ? (
              <span className="text-muted-foreground font-normal ml-2 text-sm">
                v{detail.manifest.version}
              </span>
            ) : null}
          </h3>
          <p className="text-sm text-muted-foreground">
            datablist sample data · warehouse + anomaly-rule demo
            {detail ? (
              <>
                {" · "}
                state=<code className="text-xs">{detail.state}</code>
                {" · "}
                enabled=<code className="text-xs">{detail.enabled}</code>
              </>
            ) : null}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setTick((t) => t + 1)}
          disabled={loading}
        >
          {loading ? "loading…" : "refresh"}
        </Button>
      </div>

      {error ? (
        <Alert variant="destructive">
          <AlertDescription>failed to load: {error}</AlertDescription>
        </Alert>
      ) : null}

      {/* Contributions */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm">Contributions</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          <ContribRow label="tools" items={tools} />
          <ContribRow label="warehouse tables" items={warehouseTables} />
          <ContribRow label="warehouse templates" items={warehouseTemplates} />
          <ContribRow label="anomaly rules" items={anomalyRules} />
          <ContribRow label="ui slots" items={exposes} />
        </CardContent>
      </Card>

      {/* Charts grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Customers by country (top 10)</CardTitle>
            <CardDescription>{EXTENSION_ID}.customers_by_country</CardDescription>
          </CardHeader>
          <CardContent>
            <CountryBarChart buckets={countryBuckets} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Low-stock products (&lt; 10)</CardTitle>
            <CardDescription>{EXTENSION_ID}.products_low_stock</CardDescription>
          </CardHeader>
          <CardContent>
            {lowStock.length === 0 ? (
              <p className="text-sm text-muted-foreground italic">no low-stock products</p>
            ) : (
              <ProductTable rows={lowStock} />
            )}
          </CardContent>
        </Card>
      </div>

      {/* Quality */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">
            Data-quality rule preview · {flagged.length} / {customers.length} flagged
          </CardTitle>
          <CardDescription>
            client mirror of {EXTENSION_ID}.customer_quality (sample via customers_sample)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {flagged.length === 0 ? (
            <p className="text-sm text-muted-foreground italic">no flags in the sample</p>
          ) : (
            <ul className="space-y-1 text-sm">
              {flagged.slice(0, 25).map(({ r, q }) => (
                <li key={r.customer_id} className="flex items-center gap-2">
                  <Badge variant="warning" className="text-[0.65rem]">{q.quality}</Badge>
                  <code className="text-xs">{r.customer_id}</code>
                  <span className="text-muted-foreground">{q.note || ""}</span>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <ProductsCrudPanel />
    </div>
  );
}

function ProductTable({ rows }: { rows: ReadonlyArray<ProductRow> }): React.ReactElement {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>SKU</TableHead>
          <TableHead>Name</TableHead>
          <TableHead className="text-right">Stock</TableHead>
          <TableHead className="text-right">Price</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((p) => (
          <TableRow key={p.internal_id}>
            <TableCell className="font-mono text-xs">{p.internal_id}</TableCell>
            <TableCell>{p.name}</TableCell>
            <TableCell className={cn("text-right", p.stock === 0 && "text-destructive font-semibold")}>
              {p.stock}
            </TableCell>
            <TableCell className="text-right">${Number(p.price ?? 0).toFixed(2)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/* --------------------------- Products CRUD -------------------------- */

interface ProductForm {
  internal_id: string;
  name: string;
  brand: string;
  category: string;
  price: string;
  currency: string;
  stock: string;
  availability: string;
  color: string;
  size: string;
}

const EMPTY_FORM: ProductForm = {
  internal_id: "",
  name: "",
  brand: "",
  category: "",
  price: "",
  currency: "",
  stock: "",
  availability: "",
  color: "",
  size: "",
};

function ProductsCrudPanel(): React.ReactElement {
  const [rows, setRows] = React.useState<ReadonlyArray<ProductRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [info, setInfo] = React.useState<string | null>(null);
  const [tick, setTick] = React.useState(0);
  const [editing, setEditing] = React.useState<string | null>(null);
  const [form, setForm] = React.useState<ProductForm>(EMPTY_FORM);
  const [busy, setBusy] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchTemplate<ProductRow>(`${EXTENSION_ID}.products_list`, { limit: 200 })
      .then((r) => {
        if (!cancelled) setRows(r);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [tick]);

  function reset() {
    setEditing(null);
    setForm(EMPTY_FORM);
  }

  function startEdit(row: ProductRow) {
    setEditing(row.internal_id);
    setForm({
      internal_id: row.internal_id,
      name: row.name ?? "",
      brand: row.brand ?? "",
      category: row.category ?? "",
      price: row.price != null ? String(row.price) : "",
      currency: (row as { currency?: string }).currency ?? "",
      stock: row.stock != null ? String(row.stock) : "",
      availability: row.availability ?? "",
      color: (row as { color?: string }).color ?? "",
      size: (row as { size?: string }).size ?? "",
    });
  }

  function buildRow(): Record<string, unknown> {
    const out: Record<string, unknown> = { internal_id: form.internal_id.trim() };
    if (form.name) out.name = form.name;
    if (form.brand) out.brand = form.brand;
    if (form.category) out.category = form.category;
    if (form.price !== "") {
      const n = Number(form.price);
      if (!Number.isFinite(n)) throw new Error("price must be a number");
      out.price = n;
    }
    if (form.currency) out.currency = form.currency;
    if (form.stock !== "") {
      const n = Number.parseInt(form.stock, 10);
      if (!Number.isFinite(n)) throw new Error("stock must be an integer");
      out.stock = n;
    }
    if (form.availability) out.availability = form.availability;
    if (form.color) out.color = form.color;
    if (form.size) out.size = form.size;
    return out;
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      const row = buildRow();
      if (!row.internal_id) throw new Error("internal_id is required");
      const tool = editing
        ? `${EXTENSION_ID}.products_update`
        : `${EXTENSION_ID}.products_create`;
      if (!editing && !row.name) throw new Error("name is required for create");
      const res = await callTool<{ operation: string; affected: number }>(tool, { row });
      setInfo(`${res.operation}: ${res.affected} row(s)`);
      reset();
      setTick((t) => t + 1);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(internal_id: string) {
    if (busy) return;
    if (!window.confirm(`Delete product ${internal_id}?`)) return;
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      const res = await callTool<{ operation: string; affected: number }>(
        `${EXTENSION_ID}.products_delete`,
        { internal_ids: [internal_id] },
      );
      setInfo(`delete: ${res.affected} row(s)`);
      if (editing === internal_id) reset();
      setTick((t) => t + 1);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-baseline justify-between gap-4">
          <div>
            <CardTitle>Products</CardTitle>
            <CardDescription>
              live CRUD over <code>com_rubix_example__products</code> via the host's warehouse_write capability.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setTick((t) => t + 1)}
            disabled={loading || busy}
          >
            {loading ? "loading…" : "refresh"}
          </Button>
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        {error ? (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : null}
        {info ? (
          <Alert variant="info">
            <AlertDescription>{info}</AlertDescription>
          </Alert>
        ) : null}

        {/* Form */}
        <Card className="bg-muted/30">
          <CardHeader className="pb-2">
            <div className="flex items-baseline justify-between">
              <CardTitle className="text-sm">
                {editing ? `Edit ${editing}` : "Create product"}
              </CardTitle>
              {editing ? (
                <Button variant="ghost" size="sm" onClick={reset} disabled={busy}>
                  cancel
                </Button>
              ) : null}
            </div>
            <CardDescription>
              {editing
                ? "PATCH-style: only filled fields are written"
                : "all fields except internal_id and name are optional"}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form
              onSubmit={onSubmit}
              className="grid grid-cols-[repeat(auto-fit,minmax(160px,1fr))] gap-3"
            >
              <FormField label="internal_id *" value={form.internal_id} disabled={!!editing}
                         onChange={(v) => setForm({ ...form, internal_id: v })} />
              <FormField label="name *"         value={form.name}         onChange={(v) => setForm({ ...form, name: v })} />
              <FormField label="brand"          value={form.brand}        onChange={(v) => setForm({ ...form, brand: v })} />
              <FormField label="category"       value={form.category}     onChange={(v) => setForm({ ...form, category: v })} />
              <FormField label="price"          value={form.price}        onChange={(v) => setForm({ ...form, price: v })} inputMode="decimal" />
              <FormField label="currency"       value={form.currency}     onChange={(v) => setForm({ ...form, currency: v })} />
              <FormField label="stock"          value={form.stock}        onChange={(v) => setForm({ ...form, stock: v })} inputMode="numeric" />
              <FormField label="availability"   value={form.availability} onChange={(v) => setForm({ ...form, availability: v })} />
              <FormField label="color"          value={form.color}        onChange={(v) => setForm({ ...form, color: v })} />
              <FormField label="size"           value={form.size}         onChange={(v) => setForm({ ...form, size: v })} />
              <div className="col-span-full flex justify-end">
                <Button type="submit" disabled={busy} size="sm">
                  {busy ? "…" : editing ? "Save changes" : "Create"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>

        {/* Table */}
        <div>
          <p className="text-sm text-muted-foreground mb-2">
            Catalog · {rows.length} row(s) · {EXTENSION_ID}.products_list
          </p>
          {rows.length === 0 ? (
            <p className="text-sm text-muted-foreground italic">
              no rows — ingest some via demo-ingest or create one above
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>SKU</TableHead>
                  <TableHead>Name</TableHead>
                  <TableHead>Brand</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead className="text-right">Stock</TableHead>
                  <TableHead className="text-right">Price</TableHead>
                  <TableHead>Avail.</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((p) => (
                  <TableRow
                    key={p.internal_id}
                    data-state={editing === p.internal_id ? "selected" : undefined}
                  >
                    <TableCell className="font-mono text-xs">{p.internal_id}</TableCell>
                    <TableCell>{p.name}</TableCell>
                    <TableCell>{p.brand ?? ""}</TableCell>
                    <TableCell>{p.category ?? ""}</TableCell>
                    <TableCell className={cn("text-right", p.stock === 0 && "text-destructive font-semibold")}>
                      {p.stock ?? ""}
                    </TableCell>
                    <TableCell className="text-right">
                      {p.price != null ? `$${Number(p.price).toFixed(2)}` : ""}
                    </TableCell>
                    <TableCell>{p.availability ?? ""}</TableCell>
                    <TableCell className="text-right space-x-1">
                      <Button variant="ghost" size="sm" onClick={() => startEdit(p)} disabled={busy} className="h-6 px-2 text-xs">
                        edit
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => onDelete(p.internal_id)} disabled={busy} className="h-6 px-2 text-xs text-destructive hover:text-destructive">
                        delete
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

/* ----------------------------- form field --------------------------- */

function FormField({
  label,
  value,
  onChange,
  disabled,
  inputMode,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  inputMode?: React.HTMLAttributes<HTMLInputElement>["inputMode"];
}): React.ReactElement {
  return (
    <div className="flex flex-col gap-1">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      <Input
        type="text"
        value={value}
        disabled={disabled}
        inputMode={inputMode}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}
