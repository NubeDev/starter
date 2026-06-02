# Icons

This directory intentionally ships **no binary icons**. Before the first
real bundle, generate them from a single source PNG:

```bash
cargo tauri icon path/to/source-1024.png
```

That writes `icon.png`, `icon.ico`, `icon.icns`, and the Android/iOS
asset sets that `tauri.conf.json` (`bundle.icon`) and the mobile
projects reference.

`tauri.conf.json` points `bundle.icon` at `icons/icon.png`. The
`icon.png` checked in here is a **throwaway 32x32 placeholder** — Tauri's
`generate_context!` macro reads it at COMPILE time (so `cargo check`
needs *a* valid RGBA PNG to exist), but it is not a real brand asset.
Replace it (and generate the platform variants) with `cargo tauri icon`
before shipping.
