// All demo/mock data for Rubix Flutter demo mode lives here.
// Edit this file to change fake user, devices, agent, metrics, etc.
// DEMO ONLY: safe to delete this entire file when removing demo mode.

import 'package:flutter/widgets.dart';
import 'package:lucide_icons/lucide_icons.dart';

/// Simple fake user record.
class DemoUser {
  const DemoUser({required this.id, required this.name, required this.email, this.avatarUrl});
  final String id;
  final String name;
  final String email;
  final String? avatarUrl;
}

enum DemoDeviceStatus { online, offline, warning, error }

class DemoDevice {
  const DemoDevice({required this.id, required this.name, required this.status, required this.location, required this.type});
  final String id;
  final String name;
  final DemoDeviceStatus status;
  final String location;
  final String type;
}

class DemoMetricPoint {
  const DemoMetricPoint({required this.timestamp, required this.value});
  final DateTime timestamp;
  final double value;
}

// ---------------------------------------------------------------------------
// Fake data

const demoUser = DemoUser(
  id: 'demo-lina',
  name: 'Lina Silvera',
  email: 'lina@nube.io',
  avatarUrl: 'https://randomuser.me/api/portraits/women/44.jpg',
);

final List<DemoDevice> demoDevices = List.generate(24, (i) {
  const statuses = [
    DemoDeviceStatus.online, DemoDeviceStatus.offline,
    DemoDeviceStatus.warning, DemoDeviceStatus.error,
  ];
  return DemoDevice(
    id: 'dev-${i + 1}',
    name: 'Demo Device ${i + 1}',
    status: statuses[i % statuses.length],
    location: 'Zone ${(i % 4) + 1}',
    type: i % 2 == 0 ? 'Sensor' : 'Controller',
  );
});

final List<DemoMetricPoint> demoEnergySeries = List.generate(48, (i) =>
  DemoMetricPoint(
    timestamp: DateTime.now().subtract(Duration(hours: 48 - i)),
    value: (100 + (i * 7) % 40 + (i.isEven ? 10 : -10)).toDouble(),
  ),
);

// ---------------------------------------------------------------------------
// Connections screen — connected-device rows (Figma node 6-79).
// Status drives the dot colour; arrow direction is purely visual.

enum DemoConnStatus { connected, warning, offline }

class DemoConnectedDevice {
  const DemoConnectedDevice({
    required this.id,
    required this.name,
    required this.type,
    required this.detail,
    required this.status,
    required this.icon,
  });
  final String id;
  final String name;
  /// e.g. 'Gateway', 'Sensor', 'Controller', 'Meter'.
  final String type;
  /// Short trailing detail — uptime, last-seen, latency.
  final String detail;
  final DemoConnStatus status;
  final IconData icon;
}

const List<DemoConnectedDevice> kDemoConnectedDevices = [
  DemoConnectedDevice(
    id: 'dev-compute-lab01',
    name: 'Rubix Compute Lab-01',
    type: 'Gateway',
    detail: '12ms',
    status: DemoConnStatus.connected,
    icon: LucideIcons.cpu,
  ),
  DemoConnectedDevice(
    id: 'dev-hvac-2f-east',
    name: 'HVAC Sensor 2F-East',
    type: 'Sensor',
    detail: '5min ago',
    status: DemoConnStatus.connected,
    icon: LucideIcons.thermometer,
  ),
  DemoConnectedDevice(
    id: 'dev-lighting-3a',
    name: 'Lighting Controller 3A',
    type: 'Controller',
    detail: '1hr ago',
    status: DemoConnStatus.warning,
    icon: LucideIcons.lightbulb,
  ),
  DemoConnectedDevice(
    id: 'dev-energy-meter-main',
    name: 'Energy Meter Main',
    type: 'Meter',
    detail: '3hr ago',
    status: DemoConnStatus.offline,
    icon: LucideIcons.gauge,
  ),
];

// Add more fake data as needed for other screens.
