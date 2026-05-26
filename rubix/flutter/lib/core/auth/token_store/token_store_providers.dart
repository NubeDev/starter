import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_mobile.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_web.dart';

final tokenStoreProvider = Provider<TokenStore>((ref) {
  if (kIsWeb) {
    return WebTokenStore();
  }
  return MobileTokenStore(const FlutterSecureStorage());
});
