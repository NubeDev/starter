/// `Diagnostic` — structured message returned by the resolver and
/// by `ActionResponse::Diagnostics`.
///
/// Mirrors `starter_ui_ir::Diagnostic` / `Severity`.
///
/// Pure Dart — no Flutter imports.
library;

enum SduiSeverity { info, warning, error }

class SduiDiagnostic {
  const SduiDiagnostic({
    required this.severity,
    required this.code,
    required this.message,
    this.location,
  });

  final SduiSeverity severity;

  /// Machine-readable code, e.g. `"binding_miss"`.
  final String code;

  /// Human-readable message.
  final String message;

  /// Optional pointer into the tree (component id or JSON Pointer).
  final String? location;

  factory SduiDiagnostic.fromJson(Map<String, Object?> map) => SduiDiagnostic(
        severity: switch (map['severity'] as String?) {
          'warning' => SduiSeverity.warning,
          'error' => SduiSeverity.error,
          _ => SduiSeverity.info,
        },
        code: map['code'] as String? ?? '',
        message: map['message'] as String? ?? '',
        location: map['location'] as String?,
      );

  Map<String, Object?> toJson() => {
        'severity': severity.name,
        'code': code,
        'message': message,
        if (location != null) 'location': location,
      };
}
