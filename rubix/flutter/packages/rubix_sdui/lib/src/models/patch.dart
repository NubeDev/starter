/// `SduiPatch` — one JSON Patch (RFC 6902) operation.
///
/// Pure Dart — no Flutter imports.
library;

class SduiPatch {
  const SduiPatch({required this.op, required this.path, this.value});

  /// `"add" | "remove" | "replace" | "move" | "copy" | "test"`.
  final String op;

  /// JSON Pointer.
  final String path;

  /// Value for add/replace/test (absent for remove).
  final Object? value;

  factory SduiPatch.fromJson(Map<String, Object?> map) => SduiPatch(
        op: map['op'] as String,
        path: map['path'] as String,
        value: map['value'],
      );

  Map<String, Object?> toJson() => {
        'op': op,
        'path': path,
        if (value != null) 'value': value,
      };
}
