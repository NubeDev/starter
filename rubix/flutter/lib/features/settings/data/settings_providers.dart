/// Riverpod providers for the app-settings (PIN, etc.) layer and the
/// session-scoped unlock flag that gates `/connections*`.
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/app.dart';
import 'package:rubix_flutter/core/storage/daos/settings_dao.dart';

final settingsDaoProvider = Provider<SettingsDao>((ref) {
  return SettingsDao(ref.watch(appDatabaseProvider));
});

/// Currently-stored connections PIN, or `null` if none is set.
final connectionsPinProvider = FutureProvider<String?>((ref) async {
  return ref.watch(settingsDaoProvider).getConnectionsPin();
});

/// Session-scoped flag: `true` once the user has entered the PIN this
/// run of the app. Reset on logout / sign-out. Persists across normal
/// in-app navigation so the user isn't re-prompted every time they
/// open the connections screen.
class PinUnlockedNotifier extends Notifier<bool> {
  @override
  bool build() => false;

  void unlock() => state = true;
  void lock() => state = false;
}

final pinUnlockedProvider =
    NotifierProvider<PinUnlockedNotifier, bool>(PinUnlockedNotifier.new);

/// Mutator: set, replace, or clear the connections PIN. Pass `null`
/// to remove it.
Future<void> setConnectionsPin(WidgetRef ref, String? pin) async {
  await ref.read(settingsDaoProvider).setConnectionsPin(pin);
  ref.invalidate(connectionsPinProvider);
  if (pin == null) {
    // Clearing the PIN implicitly unlocks future navigations.
    ref.read(pinUnlockedProvider.notifier).lock();
  }
}
