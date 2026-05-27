/// Accent resolution for KPI tiles and chart series.
///
/// Dart port of `packages/starter-ui-sdui-react/src/renderer/accent.ts`.
/// The hash function and intent map must stay identical so the same
/// IR renders with the same accent on web and Flutter.
library;

import '../sdui_theme.dart';

const _intentMap = <String, SduiAccent>{
  'primary':  SduiAccent.leaf,
  'positive': SduiAccent.leaf,
  'good':     SduiAccent.leaf,
  'info':     SduiAccent.sky,
  'warn':     SduiAccent.warn,
  'warning':  SduiAccent.warn,
  'energy':   SduiAccent.sun,
  'cool':     SduiAccent.aqua,
};

const _autoRotation = <SduiAccent>[
  SduiAccent.leaf,
  SduiAccent.aqua,
  SduiAccent.sun,
  SduiAccent.sky,
];

int _hash(String s) {
  var h = 0;
  for (var i = 0; i < s.length; i++) {
    // Match JS `(h * 31 + c) | 0` — clamp to 32-bit signed.
    h = (h * 31 + s.codeUnitAt(i)).toSigned(32);
  }
  return h.abs();
}

/// Returns the [SduiAccent] for a component node.
///
/// Resolution order: explicit `accent` → mapped `intent` → hash of
/// `id` (skips `warn`, reserved for explicit intent).
SduiAccent resolveAccent(Map<String, Object?> node) {
  final accent = node['accent'];
  if (accent is String) {
    for (final v in SduiAccent.values) {
      if (v.name == accent) return v;
    }
  }
  final intent = node['intent'];
  if (intent is String && _intentMap.containsKey(intent)) {
    return _intentMap[intent]!;
  }
  final id = node['id'];
  final seed = id is String ? id : '';
  return _autoRotation[_hash(seed) % _autoRotation.length];
}

/// Rotation by index — used by `kpi_grid` so siblings read
/// leaf → aqua → sun → sky in order.
SduiAccent accentByIndex(int i) => _autoRotation[i % _autoRotation.length];
