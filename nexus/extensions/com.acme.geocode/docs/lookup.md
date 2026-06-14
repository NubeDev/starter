# geocode.lookup

Resolve a free-form `address` string to `{ lat, lon }`. Deterministic: the same
address always yields the same coordinates (a stable hash spread across a valid
lat/lon range), so the result is reproducible and side-effect-free — ideal as a
peer-callable building block.
