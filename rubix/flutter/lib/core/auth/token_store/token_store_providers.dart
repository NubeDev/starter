import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_mobile.dart';
// Conditional: the real WebTokenStore imports `package:web`, which
// drags in `dart:js_interop` — fine in a real web build but it makes
// VM tests / native compiles blow up. The stub satisfies the static
// type checker on those targets; kIsWeb prevents it from ever running.
import 'package:rubix_flutter/core/auth/token_store/token_store_web_stub.dart'
    if (dart.library.js_interop)
        'package:rubix_flutter/core/auth/token_store/token_store_web.dart';

final tokenStoreProvider = Provider<TokenStore>((ref) {
  if (kIsWeb) {
    return WebTokenStore();
  }
  return MobileTokenStore(const FlutterSecureStorage());
});
