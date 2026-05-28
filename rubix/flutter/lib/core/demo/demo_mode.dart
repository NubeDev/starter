/// Demo / mock mode — when enabled, key providers short-circuit to
/// hardcoded fake data so the entire UI renders without a live
/// rubix-agent connection. Intended for design review and offline demos.
///
/// Persisted in [SharedPreferences] under [_demoKey]. Boot-loaded from
/// `main.dart` so the value is available before the first frame.
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/home/domain/me_response/me_response.dart';

const _demoKey = 'demo_mode';

/// Whether demo mode is currently on. Persists across launches.
final demoModeProvider =
    NotifierProvider<DemoModeNotifier, bool>(DemoModeNotifier.new);

class DemoModeNotifier extends Notifier<bool> {
  @override
  bool build() => false;

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    state = prefs.getBool(_demoKey) ?? false;
  }

  Future<void> set(bool on) async {
    state = on;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_demoKey, on);
  }

  Future<void> toggle() => set(!state);
}

// ---------------------------------------------------------------------------
// Hardcoded fakes — the canonical demo payloads.
// ---------------------------------------------------------------------------

/// The fake "active connection" shown across the app when demo mode is on.
Connection get kDemoConnection => Connection(
      id: -1,
      label: 'Demo Site — Sydney HQ',
      baseUrl: 'demo://nube.local',
      createdAt: DateTime(2025),
      lastUsedAt: DateTime.now(),
    );

/// The fake signed-in user.
const kDemoMe = MeResponse(
  subject: 'demo-operator',
  email: 'ops@nube.io',
  role: 'operator',
);

/// Hardcoded dashboard list — shape returned by `rubix.dashboard.list`.
const kDemoDashboards = <({String pageId, String title})>[
  (pageId: 'fleet-overview', title: 'Fleet Overview'),
  (pageId: 'energy-hvac', title: 'Energy & HVAC'),
  (pageId: 'active-alerts', title: 'Active Alerts'),
  (pageId: 'rooms-sensors', title: 'Rooms & Sensors'),
];
