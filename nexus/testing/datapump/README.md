# Nexus Datapump

Publishes no-auth MQTT or Zenoh test telemetry shaped like the Rubix warehouse
histories described in `rubix/extensions/com.nubeio.rubixos/DB.md`.

MQTT test broker:

```bash
make mqtt-broker
```

Zenoh test router:

```bash
make zenoh-broker
```

Run with MQTT:

```bash
make run-mqtt
```

Run with Zenoh:

```bash
make run-zenoh
```

Useful overrides:

```bash
make run-zenoh \
  PATH_PREFIX=rubix/testing \
  TENANT_ID='*' \
  SITES=4 \
  METERS_PER_KIND=8 \
  INTERVAL_MS=1000 \
  COUNT=500
```

Smoke-test both transports:

```bash
make smoke-mqtt
make smoke-zenoh
```

Payloads are JSON and include:

- `tenant_id`
- `site_id`
- `host_uuid`
- `point_uuid`
- `meter_id`
- `kind` (`elec` or `water`)
- `secondary_tag` (`power` or `reading`)
- `value`
- `timestamp`

MQTT topics and Zenoh key expressions use this form:

```text
{path-prefix}/{path-tenant}/{site_id}/{kind}/{meter_id}
```

`--transport` can also be set with `DATAPUMP_TRANSPORT=mqtt|zenoh`. The
misspelling `zenho` is accepted as an alias. The path tenant defaults to `all`
because MQTT topics cannot safely use `*`; the payload `tenant_id` still
defaults to `*` to match the Rubix warehouse fixture convention.
