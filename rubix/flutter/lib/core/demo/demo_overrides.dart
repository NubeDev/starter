/// Demo-mode provider overrides — applied when `DEMO_MODE=true` at boot
/// so every screen renders fully populated without a live rubix-agent.
///
/// Keep this file isolated. When the flag is off the real provider tree
/// is untouched; we never patch app behaviour.
///
/// DEMO ONLY: safe to delete this file when removing demo mode.
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:rubix_flutter/core/demo/demo_mode.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';

/// Builds the root [ProviderContainer] with all demo-mode overrides
/// attached. All future/notifier providers consumed by the redesigned
/// screens are short-circuited to canned data; downstream Dio / API
/// calls never run.
///
/// The list of overrides lives inline here because riverpod's `Override`
/// sealed class is not part of its public export surface, so we let
/// type inference flow from `ProviderContainer.overrides`.
ProviderContainer buildDemoContainer() {
  return ProviderContainer(
    overrides: [
      activeConnectionProvider.overrideWith((ref) async => kDemoConnection),
      agentHealthProvider.overrideWith((ref) async => const AgentHealthOk()),
      currentUserProvider.overrideWith((ref) async => kDemoMe),
      authControllerProvider.overrideWith(_DemoAuthController.new),
      connectionListControllerProvider
          .overrideWith(_DemoConnectionListController.new),
      connectionsPinProvider.overrideWith((ref) async => null),
    ],
  );
}

class _DemoAuthController extends AuthController {
  @override
  Future<AuthState> build() async =>
      const AuthAuthenticated('demo-token');
}

class _DemoConnectionListController extends ConnectionListController {
  @override
  Future<List<Connection>> build() async => [kDemoConnection];
}
