import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:rubix_flutter/app.dart';
import 'package:rubix_flutter/core/demo/demo_mode.dart';
import 'package:rubix_flutter/core/demo/demo_overrides.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  final container =
      demoMode ? buildDemoContainer() : ProviderContainer();
  await container.read(themeModeProvider.notifier).load();
  await container.read(localeProvider.notifier).load();

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const RubixApp(),
    ),
  );
}
