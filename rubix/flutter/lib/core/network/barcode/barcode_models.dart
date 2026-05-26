/// Barcode/QR code scanning result model.
class BarcodeResult {
  /// The raw decoded value from the barcode/QR code.
  final String value;

  /// The format of the barcode (e.g. qr, ean13, code128).
  final BarcodeFormat format;

  /// Timestamp when the barcode was scanned.
  final DateTime scannedAt;

  BarcodeResult({
    required this.value,
    this.format = BarcodeFormat.unknown,
    DateTime? scannedAt,
  }) : scannedAt = scannedAt ?? DateTime.now();

  @override
  String toString() => 'BarcodeResult($format: $value)';
}

/// Supported barcode formats.
enum BarcodeFormat {
  qr,
  ean8,
  ean13,
  code39,
  code93,
  code128,
  itf,
  upcA,
  upcE,
  pdf417,
  aztec,
  dataMatrix,
  unknown;
}
