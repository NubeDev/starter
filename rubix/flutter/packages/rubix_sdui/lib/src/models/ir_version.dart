/// IR protocol version this client can render.
///
/// Mirrors `IR_VERSION` in `crates/starter-ui-ir/src/lib.rs`.
/// Bump only when a breaking IR change lands AND the renderer
/// ships matching support.
library;

const int kSupportedIrVersion = 5;
