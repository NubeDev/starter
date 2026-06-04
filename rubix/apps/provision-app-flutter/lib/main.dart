import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/app.dart';
import 'package:provision_app/core/network/network_providers.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/auth/data/auth_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  final container = ProviderContainer();

  // Load the persisted theme, then hydrate the transport (base URL + token from
  // the keychain) and restore any existing session — all before first frame so
  // the gate doesn't flicker.
  await container.read(themeKeyProvider.notifier).load();
  await container.read(transportProvider).hydrate();
  await container.read(authControllerProvider.notifier).restore();

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const ProvisionApp(),
    ),
  );
}
