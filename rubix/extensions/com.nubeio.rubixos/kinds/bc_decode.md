# `com.nubeio.rubixos.bc_decode`

Decode a scanned barcode string into a `ScannedIdentity` — model,
network, address, default IP and hardware revision. Pure and
stateless: it touches no database. The matched device template is
resolved from the parsed model so the caller can preview points and
the widget group before provisioning.
