# Example extension — kinds/

Per-kind YAML manifests an extension contributes via
`contributes.nodes` in `block.yaml`. Empty for the reference
extension — it only ships tools / skills / flows.

A real block contributing a node kind drops one YAML per kind
here:

```
kinds/
├── com.acme.mqtt.client.yaml
└── com.acme.mqtt.subscription.yaml
```

See the planned `rubix-extensions-sdk` (an upstream item — see
[docs/design/starter-changes/README.md](../../docs/design/starter-changes/README.md))
for the schema.
