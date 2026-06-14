# `com.acme.devices.sensor_register`

Registers a sensor against the `device_id` produced by `device.create`.
Idempotent on `device_id` (DOCS §8c) so resume re-entry never double-registers.
