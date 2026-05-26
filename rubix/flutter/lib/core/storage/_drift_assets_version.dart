/// The drift release tag the files in web/ came from.
/// Bump when web/sqlite3.wasm or web/drift_worker.dart.js is
/// refreshed from a different release.
const driftAssetsReleaseTag = '2.28.0';

/// The range of drift dependency versions known to be wasm-ABI
/// compatible with the assets above. Inclusive on both ends.
/// Widen as new drift releases ship without wasm-side changes;
/// narrow (i.e. force an asset refresh) when drift's release
/// notes mention sqlite3 wasm, worker, or web schema changes.
const driftAssetsCompatRange = (min: '2.28.0', max: '2.30.99');
