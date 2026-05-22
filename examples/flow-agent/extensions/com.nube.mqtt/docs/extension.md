# MQTT extension for `flow-agent`

This bundle is the load-bearing demo for slice B of
`DOCS/extensions/scope/FLOW-NODES.md` — a hot-loadable extension
that contributes two flow node kinds (`com.nube.mqtt.publish` and
`com.nube.mqtt.subscribe`) over the new
[`flow.node.invoke`](../../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs)
wire method.

The driver opens **one persistent MQTT connection per child
process** and demuxes invocations on top of it — extensions are
stateful processes; nodes are stateless behaviours on top
(R-flow-node-8).

See `docs/publish.md` and `docs/subscribe.md` for the per-kind
contract.
