# `com.nube.mqtt.subscribe`

Streaming subscription. Emits one `stream.event` per inbound MQTT
message; terminates on `stream.cancel` (sent by the host when the
flow run is cancelled).

## Settings

| field | type    | required | description                              |
| ----- | ------- | -------- | ---------------------------------------- |
| topic | string  | yes      | Topic filter (supports `+` and `#`).     |
| qos   | int     | no       | QoS 0 / 1 / 2 (default 0).               |

## Stream event payload

```json
{
  "topic": "demo/sensor/temp",
  "payload": "23.4",
  "retain": false,
  "qos": 0
}
```

Sent verbatim — the host's engine wraps each event into a
[`SlotValue::Json`](../../../../crates/starter-flow-spi/src/node.rs).
