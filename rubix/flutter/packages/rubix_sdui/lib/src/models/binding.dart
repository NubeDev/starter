/// `BindingSpec` — slot binding wire form.
///
/// Mirrors the Rust `BindingSpec` in `crates/starter-ui-ir`. Two
/// forms on the wire:
///
/// - [ShortBinding]: a bare string, e.g. `"$target.enabled"`.
/// - [FullBinding]:  `{ slot, concurrency?, debounce_ms? }`.
///
/// Pure Dart — no Flutter imports.
library;

enum SduiConcurrency { lww, occ }

sealed class BindingSpec {
  const BindingSpec();

  String get slotExpr;
  SduiConcurrency get concurrency;
  int? get debounceMs;

  factory BindingSpec.fromJson(Object? json) {
    if (json is String) return ShortBinding(json);
    if (json is Map<String, Object?>) return FullBinding.fromJson(json);
    throw ArgumentError('BindingSpec: expected String or Map, got $json');
  }

  Object toJson();
}

final class ShortBinding extends BindingSpec {
  const ShortBinding(this.slot);
  final String slot;

  @override
  String get slotExpr => slot;
  @override
  SduiConcurrency get concurrency => SduiConcurrency.lww;
  @override
  int? get debounceMs => null;

  @override
  String toJson() => slot;
}

final class FullBinding extends BindingSpec {
  const FullBinding({
    required this.slot,
    this.concurrency = SduiConcurrency.lww,
    this.debounceMs,
  });

  final String slot;
  @override
  final SduiConcurrency concurrency;
  @override
  final int? debounceMs;

  @override
  String get slotExpr => slot;

  factory FullBinding.fromJson(Map<String, Object?> map) => FullBinding(
        slot: map['slot'] as String,
        concurrency: switch (map['concurrency'] as String?) {
          'occ' => SduiConcurrency.occ,
          _ => SduiConcurrency.lww,
        },
        debounceMs: (map['debounce_ms'] as num?)?.toInt(),
      );

  @override
  Map<String, Object?> toJson() => {
        'slot': slot,
        'concurrency': concurrency.name,
        if (debounceMs != null) 'debounce_ms': debounceMs,
      };
}
