import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/router/app_router/app_router.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';

/// Global keepAlive provider for the Drift database (native only).
///
/// Must never be resolved on web — the web build routes every data
/// call through `rubix_server` over REST. The kIsWeb fork lives in
/// `lib/core/storage/data_layer.dart`; this guard is a tripwire in
/// case some new caller forgets to go through it.
final appDatabaseProvider = Provider<AppDatabase>((ref) {
  if (kIsWeb) {
    throw StateError(
      'appDatabaseProvider must not be read on web — use the '
      'repository providers in core/storage/data_layer.dart instead.',
    );
  }
  final db = AppDatabase();
  ref.onDispose(db.close);
  return db;
});

class RubixApp extends ConsumerStatefulWidget {
  const RubixApp({super.key});

  @override
  ConsumerState<RubixApp> createState() => _RubixAppState();
}

class _RubixAppState extends ConsumerState<RubixApp> {
  @override
  void initState() {
    super.initState();
    // Load persisted settings.
    ref.read(themeModeProvider.notifier).load();
    ref.read(localeProvider.notifier).load();
  }

  @override
  Widget build(BuildContext context) {
    final router = ref.watch(appRouterProvider);
    final themeMode = ref.watch(themeModeProvider);
    final locale = ref.watch(localeProvider);

    return MaterialApp.router(
      title: 'Rubix',
      theme: rubixLightTheme,
      darkTheme: rubixDarkTheme,
      themeMode: themeMode,
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      routerConfig: router,
    );
  }
}
