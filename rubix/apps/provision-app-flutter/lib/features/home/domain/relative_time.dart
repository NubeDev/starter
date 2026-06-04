/// Compact relative-time label (e.g. `2m`, `1h`, `3d`) for a timestamp string,
/// used by the Home "Recent" list. Parses an ISO-8601 [iso] (the device's
/// `provisioned_at`) and renders the elapsed time in the largest sensible unit.
/// Returns null when the input is null/blank/unparseable so callers can decide
/// how to render a missing time.
String? relativeTimeLabel(String? iso, {DateTime? now}) {
  if (iso == null || iso.trim().isEmpty) return null;
  final then = DateTime.tryParse(iso);
  if (then == null) return null;

  final reference = now ?? DateTime.now();
  var delta = reference.difference(then);
  if (delta.isNegative) delta = Duration.zero; // clock skew → "now"

  if (delta.inMinutes < 1) return 'now';
  if (delta.inMinutes < 60) return '${delta.inMinutes}m';
  if (delta.inHours < 24) return '${delta.inHours}h';
  if (delta.inDays < 7) return '${delta.inDays}d';
  if (delta.inDays < 30) return '${(delta.inDays / 7).floor()}w';
  if (delta.inDays < 365) return '${(delta.inDays / 30).floor()}mo';
  return '${(delta.inDays / 365).floor()}y';
}
