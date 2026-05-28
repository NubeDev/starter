// Demo/mock repositories for Rubix Flutter demo mode.
// Each repository returns fake data from demo_data.dart.
// DEMO ONLY: safe to delete this entire file when removing demo mode.

import 'demo_data.dart';

class DemoUserRepository {
  Future<DemoUser> getCurrentUser() async => demoUser;
}

class DemoDeviceRepository {
  Future<List<DemoDevice>> getDevices() async => demoDevices;
}

class DemoMetricRepository {
  Future<List<DemoMetricPoint>> getEnergySeries() async => demoEnergySeries;
}

// Add more demo repositories as needed for other screens.
