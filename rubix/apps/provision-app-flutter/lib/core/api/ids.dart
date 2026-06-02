import 'dart:math';

/// Client-side id minting for new rows. Mirrors the React `api/ids.ts`:
/// `${prefix}_${base36 timestamp}${8 random hex}`.
final _rand = Random();

String mintId(String prefix) {
  final ts = DateTime.now().millisecondsSinceEpoch.toRadixString(36);
  final rnd = _rand.nextInt(0xFFFFFFFF).toRadixString(16).padLeft(8, '0');
  return '${prefix}_$ts$rnd';
}
