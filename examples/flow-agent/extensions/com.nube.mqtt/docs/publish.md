# `com.nube.mqtt.publish`

Publish one message to the configured MQTT broker.

## Settings

| field   | type    | required | description                              |
| ------- | ------- | -------- | ---------------------------------------- |
| topic   | string  | yes      | Topic to publish to.                     |
| qos     | int     | no       | QoS 0 / 1 / 2 (default 0).               |
| retain  | bool    | no       | Retain flag (default false).             |

## Input slots

| slot    | type    | description                              |
| ------- | ------- | ---------------------------------------- |
| payload | bytes / | Message body. Bytes are sent verbatim;   |
|         | string  | strings are UTF-8 encoded.               |

## Output slots

| slot          | type | description                        |
| ------------- | ---- | ---------------------------------- |
| published_at  | int  | Wall-clock millis when broker      |
|               |      | acked the publish.                 |
