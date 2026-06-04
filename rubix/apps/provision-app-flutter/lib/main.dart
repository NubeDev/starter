import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/app.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/demo_bc_api.dart';
import 'package:provision_app/core/network/network_providers.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/auth/data/auth_controller.dart';

/// TEMPORARY: `--dart-define=DNA_DEMO=true` opens the gate with mock data so the
/// gated screens can be previewed without a live agent. Remove before merging.
const _dnaDemo = bool.fromEnvironment('DNA_DEMO');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  final container = ProviderContainer(
    overrides: [
      if (_dnaDemo)
        bcApiProvider.overrideWith(
          (ref) => DemoBcApi(ref.watch(transportProvider)),
        ),
    ],
  );

  // Load the persisted theme, then hydrate the transport (base URL + token from
  // the keychain) and restore any existing session — all before first frame so
  // the gate doesn't flicker.
  await container.read(themeKeyProvider.notifier).load();
  if (_dnaDemo) {
    container.read(authControllerProvider.notifier).seedDemo();
  } else {
    await container.read(transportProvider).hydrate();
    await container.read(authControllerProvider.notifier).restore();
  }

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const ProvisionApp(),
    ),
  );
}
