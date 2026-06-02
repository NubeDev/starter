import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart' as ms;
import 'package:mobile_scanner/mobile_scanner.dart' show BarcodeCapture, MobileScanner, MobileScannerController, TorchState;

import 'package:rubix_flutter/core/network/barcode/barcode_models.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

export 'barcode_models.dart';

/// A reusable full-screen barcode/QR scanner screen.
///
/// Returns the scanned [BarcodeResult] via `Navigator.pop` when the user
/// confirms a scan. Returns `null` if dismissed without scanning.
///
/// Usage:
/// ```dart
/// final result = await Navigator.of(context).push<BarcodeResult>(
///   MaterialPageRoute(builder: (_) => const BarcodeScannerScreen()),
/// );
/// if (result != null) print('Scanned: ${result.value}');
/// ```
class BarcodeScannerScreen extends StatefulWidget {

  const BarcodeScannerScreen({
    super.key,
    this.title = 'Scan Code',
    this.confirmLabel = 'Use Code',
    this.rescanLabel = 'Scan Again',
  });
  /// Title shown in the top bar.
  final String title;

  /// Label for the confirm button.
  final String confirmLabel;

  /// Label for the re-scan button.
  final String rescanLabel;

  @override
  State<BarcodeScannerScreen> createState() => _BarcodeScannerScreenState();
}

class _BarcodeScannerScreenState extends State<BarcodeScannerScreen> {
  final MobileScannerController _controller = MobileScannerController();

  bool _hasScanned = false;
  BarcodeResult? _result;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _onDetect(BarcodeCapture capture) {
    if (_hasScanned) return;
    final barcode = capture.barcodes.firstOrNull;
    if (barcode == null || barcode.rawValue == null) return;

    setState(() {
      _hasScanned = true;
      _result = BarcodeResult(
        value: barcode.rawValue!,
        format: _mapFormat(barcode.format),
        scannedAt: DateTime.now(),
      );
    });

    HapticFeedback.mediumImpact();
  }

  void _resetScan() {
    setState(() {
      _hasScanned = false;
      _result = null;
    });
  }

  static BarcodeFormat _mapFormat(ms.BarcodeFormat format) {
    switch (format) {
      case ms.BarcodeFormat.qrCode:
        return BarcodeFormat.qr;
      case ms.BarcodeFormat.ean8:
        return BarcodeFormat.ean8;
      case ms.BarcodeFormat.ean13:
        return BarcodeFormat.ean13;
      case ms.BarcodeFormat.code39:
        return BarcodeFormat.code39;
      case ms.BarcodeFormat.code93:
        return BarcodeFormat.code93;
      case ms.BarcodeFormat.code128:
        return BarcodeFormat.code128;
      case ms.BarcodeFormat.itf14:
        return BarcodeFormat.itf;
      case ms.BarcodeFormat.upcA:
        return BarcodeFormat.upcA;
      case ms.BarcodeFormat.upcE:
        return BarcodeFormat.upcE;
      case ms.BarcodeFormat.pdf417:
        return BarcodeFormat.pdf417;
      case ms.BarcodeFormat.aztec:
        return BarcodeFormat.aztec;
      case ms.BarcodeFormat.dataMatrix:
        return BarcodeFormat.dataMatrix;
      case _:
        return BarcodeFormat.unknown;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: Colors.black,
      body: Stack(
        children: [
          // Camera
          if (!_hasScanned)
            MobileScanner(
              controller: _controller,
              onDetect: _onDetect,
              errorBuilder: (context, error) {
                return Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.videocam_off,
                          size: 48, color: colorScheme.error),
                      const SizedBox(height: 16),
                      Text(
                        'Camera error',
                        style: theme.textTheme.titleMedium
                            ?.copyWith(color: Colors.white),
                      ),
                      const SizedBox(height: 8),
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 32),
                        child: Text(
                          error.errorDetails?.message ??
                              'Could not access camera',
                          style: theme.textTheme.bodySmall
                              ?.copyWith(color: Colors.white70),
                          textAlign: TextAlign.center,
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),

          // Viewfinder overlay
          if (!_hasScanned) _buildOverlay(colorScheme),

          // Top bar
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(4, 4, 4, 0),
              child: Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.arrow_back, size: 22),
                    color: Colors.white,
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                  const Spacer(),
                  Text(
                    widget.title,
                    style: theme.textTheme.titleMedium
                        ?.copyWith(color: Colors.white),
                  ),
                  const Spacer(),
                  if (!_hasScanned)
                    ValueListenableBuilder(
                      valueListenable: _controller,
                      builder: (context, state, _) {
                        return IconButton(
                          icon: Icon(
                            state.torchState == TorchState.on
                                ? Icons.flash_off
                                : Icons.flash_on,
                            size: 20,
                          ),
                          color: Colors.white,
                          onPressed: _controller.toggleTorch,
                        );
                      },
                    )
                  else
                    const SizedBox(width: 48),
                ],
              ),
            ),
          ),

          // Result card
          if (_hasScanned && _result != null)
            _buildResultCard(theme, colorScheme),
        ],
      ),
    );
  }

  Widget _buildOverlay(ColorScheme colorScheme) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          SizedBox(
            width: 260,
            height: 260,
            child: CustomPaint(
              painter: _ViewfinderPainter(
                cornerColor: colorScheme.primary,
              ),
            ),
          ),
          const SizedBox(height: 24),
          const Text(
            'Point camera at a barcode or QR code',
            style: TextStyle(fontSize: 14, color: Colors.white70),
          ),
        ],
      ),
    );
  }

  Widget _buildResultCard(ThemeData theme, ColorScheme colorScheme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: NubeCard(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.check_circle, size: 48, color: colorScheme.primary),
              const SizedBox(height: 16),
              Text('Code Scanned', style: theme.textTheme.titleMedium),
              const SizedBox(height: 12),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SelectableText(
                  _result!.value,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                  ),
                  maxLines: 4,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Format: ${_result!.format.name}',
                style: theme.textTheme.labelSmall,
              ),
              const SizedBox(height: 20),
              Row(
                children: [
                  Expanded(
                    child: NubeButton(
                      label: widget.rescanLabel,
                      onPressed: _resetScan,
                      variant: NubeButtonVariant.outline,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: NubeButton(
                      label: widget.confirmLabel,
                      onPressed: () => Navigator.of(context).pop(_result),
                      variant: NubeButtonVariant.ghost,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Draws corner brackets for the viewfinder.
class _ViewfinderPainter extends CustomPainter {

  _ViewfinderPainter({required this.cornerColor});
  final Color cornerColor;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = cornerColor
      ..strokeWidth = 4
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    const cl = 32.0;
    final w = size.width;
    final h = size.height;

    canvas
      ..drawLine(Offset.zero, const Offset(cl, 0), paint)
      ..drawLine(Offset.zero, const Offset(0, cl), paint)
      ..drawLine(Offset(w, 0), Offset(w - cl, 0), paint)
      ..drawLine(Offset(w, 0), Offset(w, cl), paint)
      ..drawLine(Offset(0, h), Offset(cl, h), paint)
      ..drawLine(Offset(0, h), Offset(0, h - cl), paint)
      ..drawLine(Offset(w, h), Offset(w - cl, h), paint)
      ..drawLine(Offset(w, h), Offset(w, h - cl), paint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
