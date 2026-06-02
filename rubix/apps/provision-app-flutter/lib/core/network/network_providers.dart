import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:provision_app/core/network/credential_store.dart';
import 'package:provision_app/core/network/transport.dart';

/// The platform credential store (base URL in prefs, token in the keychain).
final credentialStoreProvider = Provider<CredentialStore>((ref) {
  return CredentialStore();
});

/// The app-wide [RubixTransport]. A singleton for the session — it holds the
/// in-flight read cache and the current token in memory. Hydrated from
/// persisted credentials at boot in `main()`.
final transportProvider = Provider<RubixTransport>((ref) {
  return RubixTransport(ref.watch(credentialStoreProvider));
});
