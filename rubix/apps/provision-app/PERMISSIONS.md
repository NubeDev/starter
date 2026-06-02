# Android permissions (manual manifest additions)

`src-tauri/gen/` is **`.gitignored` and regenerated** by `cargo tauri android init`.
Tauri v2 has no manifest-overlay/config knob for app permissions, so the
following additions live directly in the generated manifest and must be
re-applied if `gen/android` is ever wiped (a clean `init`). Normal
`build`/`dev`/incremental `init` runs preserve them.

## File
`src-tauri/gen/android/app/src/main/AndroidManifest.xml`

## Additions (just under the existing `INTERNET` permission)

```xml
<uses-permission android:name="android.permission.CAMERA" />
<uses-feature android:name="android.hardware.camera" android:required="false" />
<uses-feature android:name="android.hardware.camera.autofocus" android:required="false" />
```

## Why

The QR/barcode scanner (`src/scan/Scanner.tsx`) uses `@zxing/browser`, which
calls `getUserMedia` in the WebView. wry's generated `RustWebChromeClient`
already handles `onPermissionRequest` — it maps the web `VIDEO_CAPTURE`
request to `Manifest.permission.CAMERA` and launches the Android runtime
permission prompt. But Android **auto-denies a runtime request for a
permission that isn't declared in the manifest**, so without the `CAMERA`
line the prompt never appears and the camera silently fails with
`NotAllowedError`.

`required="false"` on the camera features keeps the app installable on
camera-less devices; the scanner degrades to manual code entry.

## Granting at test time

First camera use shows the OS prompt. To pre-grant (or recover from a prior
"Deny") without tapping through:

```bash
adb shell pm grant com.nubeio.rubixos.provision android.permission.CAMERA
# revoke (to re-test the prompt):
adb shell pm revoke com.nubeio.rubixos.provision android.permission.CAMERA
```
