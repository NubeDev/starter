# com.acme.devices

The Setup / Automation Builder example extension
(`DOCS/setup-automation-builder.md` §9 "Custom logic via extensions").
**Process flavour** — a supervised child the host spawns, health-checks, and
restarts — modelled on [`com.nexus.hello`](../com.nexus.hello).

It demonstrates every extension seam the automation builder adds:

- **Custom node kinds** (`contributes.nodes[]` + `contributes.tools[]`):
  `com.acme.devices.device_create` and `com.acme.devices.sensor_register` — the
  domain side-effect steps the automation calls. Both are **idempotent on a
  natural key** (the scanned barcode / the device id, DOCS §8c) so resume
  re-entry after a failed step never double-provisions hardware. Idempotency is
  achieved the robust way: the output id is a pure function of the natural key,
  so there is no shared state to corrupt across process restarts.
- **Bundled setup template** (`contributes.setup_templates[]`):
  [`templates/add-device.yaml`](../templates/add-device.yaml) — the "Add a
  device" automation. On enable the host imports it into the `TemplateStore`
  with `source = Extension { ext_id }`, through the same path REST
  `/setup/templates/import` uses (envelope → nested `flow` → `FlowBody` →
  node-kind validation). Disabling the extension removes it.
- **Verify-page query-kind** (`contributes.warehouse_templates[]`):
  `com.acme.devices.site_checkout`, scoped by the `$caller_team_ids` host token
  (P3a) so an installer sees only their site's rows.
- **UI** (`contributes.ui`): a sidebar nav entry + a `main`-slot "Provision
  device" page that drives the full barcode → run → SSE-progress → resume loop
  against the host's `/setup` REST surface.

## Trusted identity (DOCS §9)

The `device_create` node reads `caller_user_id` / `caller_team_ids` /
`caller_tenant_id` from the **server-seeded trusted slots** the run service
writes from the verified `Principal` at launch — never from form input. The
template's `input_bindings` cannot target those reserved slots, so an installer
can't spoof which site/owner a device is tagged to.

## Build & install

```sh
make build      # cargo build the process binary + vite build the UI
make pack       # tar the bundle
make install    # POST /api/v1/extensions/install (then restart nexus-api)
make load       # enable
make test       # e2e probe: list, detail, ui bytes, run the device tools
```

The in-repo copy under `NEXUS_EXTENSIONS_DIR` is scanned at boot, so on a dev
stack you only need `make build` + a restart to surface it live.
