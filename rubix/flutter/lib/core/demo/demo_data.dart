// All demo/mock data for Rubix Flutter demo mode lives here.
// Edit this file to change fake user, devices, agent, metrics, etc.
// DEMO ONLY: safe to delete this entire file when removing demo mode.

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

// Add more fake data as needed for other screens.
