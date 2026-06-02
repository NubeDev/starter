import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

/// A shared refresh signal: every mutation bumps it, list controllers watch it
/// and re-fetch. Ported from the React app's `api/refresh.ts` (a monotonically
/// increasing version + staggered re-bumps).
///
/// Fires immediately, then staggered re-bumps. A read issued microseconds after
/// a write can land on a pooled DB connection that hasn't observed the commit
/// yet (the read-after-write window — ~100ms, occasionally a few seconds). The
/// delayed passes re-read once the write is visible, so lists converge without
/// the user reloading. Reads are cheap and fresh-deduped, so over-firing is
/// harmless.
final refreshProvider =
    NotifierProvider<RefreshNotifier, int>(RefreshNotifier.new);

class RefreshNotifier extends Notifier<int> {
  @override
  int build() => 0;

  void bump() {
    state = state + 1;
    for (final ms in const [300, 800, 1500, 3000, 5000]) {
      Timer(Duration(milliseconds: ms), () => state = state + 1);
    }
  }
}
