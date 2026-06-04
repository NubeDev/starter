import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/primary_button.dart';

/// Camera viewport (glass-framed, animated scan-line) + a manual fallback to
/// paste/wedge a `rubix://add?…` URL or serial. Uses `mobile_scanner` for
/// QR/Code128 — replaces the React `@zxing/browser` Scanner. Emits the raw
/// decoded string upward; the flow decodes it via `bc_decode`.
class Scanner extends ConsumerStatefulWidget {
  const Scanner({required this.onCode, super.key});

  final ValueChanged<String> onCode;

  @override
  ConsumerState<Scanner> createState() => _ScannerState();
}

class _ScannerState extends ConsumerState<Scanner>
    with SingleTickerProviderStateMixin {
  // Constrain decoding to the formats we actually print (QR + Code128) so each
  // frame's decode budget is spent on the right patterns.
  final MobileScannerController _controller = MobileScannerController(
    formats: const [BarcodeFormat.qrCode, BarcodeFormat.code128],
  );
  late final AnimationController _sweep;
  final TextEditingController _manual = TextEditingController();
  bool _handled = false;

  @override
  void initState() {
    super.initState();
    _sweep = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 2200),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _sweep.dispose();
    _controller.dispose();
    _manual.dispose();
    super.dispose();
  }

  void _onDetect(BarcodeCapture capture) {
    if (_handled) return;
    final raw = capture.barcodes.isEmpty
        ? null
        : capture.barcodes.first.rawValue;
    if (raw == null || raw.isEmpty) return;
    _handled = true;
    _controller.stop();
    widget.onCode(raw);
  }

  void _submitManual() {
    final v = _manual.text.trim();
    if (v.isNotEmpty) widget.onCode(v);
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // ── camera viewport (no backdrop-filter — a live preview must not be
        // nested under a blur or the compositor freezes the frame) ──────────
        AspectRatio(
          aspectRatio: 1,
          child: Container(
            clipBehavior: Clip.antiAlias,
            decoration: BoxDecoration(
              color: Colors.black,
              borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
              border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
              boxShadow: [
                BoxShadow(
                  color: look.accent.withValues(alpha: 0.2),
                  spreadRadius: 1,
                ),
                BoxShadow(
                  color: look.accent.withValues(alpha: 0.5),
                  blurRadius: 50,
                  offset: const Offset(0, 18),
                  spreadRadius: -20,
                ),
              ],
            ),
            child: Stack(
              fit: StackFit.expand,
              children: [
                MobileScanner(
                  controller: _controller,
                  onDetect: _onDetect,
                  errorBuilder: (context, error) =>
                      _CameraError(message: _describe(error)),
                ),
                // sweeping scan line
                IgnorePointer(
                  child: AnimatedBuilder(
                    animation: _sweep,
                    builder: (context, child) {
                      return Align(
                        alignment: Alignment(0, _sweep.value * 2 - 1),
                        child: child,
                      );
                    },
                    child: Container(
                      margin: const EdgeInsets.symmetric(horizontal: 24),
                      height: 2,
                      decoration: BoxDecoration(
                        color: look.accent,
                        borderRadius: BorderRadius.circular(999),
                        boxShadow: [
                          BoxShadow(
                            color: look.accent,
                            blurRadius: 12,
                            spreadRadius: 2,
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                // corner brackets
                IgnorePointer(
                  child: Padding(
                    padding: const EdgeInsets.all(20),
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(RubixTokens.radius),
                        border: Border.all(
                          color: Colors.white.withValues(alpha: 0.3),
                          width: 2,
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 12,
                  top: 12,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.black.withValues(alpha: 0.4),
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(LucideIcons.camera, size: 14, color: look.accent),
                        const SizedBox(width: 6),
                        Text(
                          'Scanning',
                          style: TextStyle(
                            color: look.ink,
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 20),

        // ── manual / wedge entry ────────────────────────────────────────────
        GlassSurface(
          borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Row(
                  children: [
                    Icon(
                      LucideIcons.keyboard,
                      size: 16,
                      color: look.inkMuted,
                    ),
                    const SizedBox(width: 8),
                    const Text('Or paste / scan a code', style: RubixText.label),
                  ],
                ),
              ),
              GlassTextField(
                controller: _manual,
                semanticLabel: 'Barcode payload',
                hint: 'rubix://add?id=DRP-9F2C18&model=droplet…',
                onSubmitted: _submitManual,
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 12),
              PrimaryButton(
                label: 'Identify device',
                accent: look.accent,
                onPressed:
                    _manual.text.trim().isEmpty ? null : _submitManual,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// Turn a [MobileScannerException] into something a field tech can act on.
String _describe(MobileScannerException error) {
  return switch (error.errorCode) {
    MobileScannerErrorCode.permissionDenied =>
      'Camera permission denied. Enable it in Settings → Apps → '
          'Rubix Provision → Permissions, then reopen the scanner.',
    MobileScannerErrorCode.unsupported =>
      'Scanning is not supported on this device.',
    _ => error.errorDetails?.message ?? 'Camera unavailable',
  };
}

class _CameraError extends StatelessWidget {
  const _CameraError({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            LucideIcons.cameraOff,
            size: 32,
            color: look.inkMuted,
          ),
          const SizedBox(height: 12),
          Text(
            message,
            textAlign: TextAlign.center,
            style: TextStyle(color: look.inkMuted, fontSize: 14),
          ),
          const SizedBox(height: 8),
          Text(
            'Use manual entry below.',
            textAlign: TextAlign.center,
            style: TextStyle(color: look.inkMuted, fontSize: 12),
          ),
        ],
      ),
    );
  }
}
