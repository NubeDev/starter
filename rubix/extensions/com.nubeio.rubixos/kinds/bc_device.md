# `com.nubeio.rubixos.bc_device`

Device lifecycle tools. `bc_device_update` renames a device,
re-places it onto a different site / location / page, or flips its
trend/alarm toggles. `bc_device_decommission` either soft-flips the
device `status` or, when `hard` is set, cascade-deletes the device
along with its points, widgets and alarms. Both are tenant-scoped.
