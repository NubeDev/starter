/// Models for network scanning results.
library;

/// A device discovered on the local network.
class ScannedHost {

  const ScannedHost({
    required this.ip,
    this.responseTime,
    this.openPorts = const [],
  });
  final String ip;
  final Duration? responseTime;
  final List<OpenPort> openPorts;

  ScannedHost copyWithPorts(List<OpenPort> ports) =>
      ScannedHost(ip: ip, responseTime: responseTime, openPorts: ports);

  @override
  String toString() =>
      'ScannedHost($ip, ports: ${openPorts.map((p) => p.port).toList()})';
}

/// An open TCP port on a host.
class OpenPort {

  const OpenPort({
    required this.port,
    required this.isOpen,
    this.responseTime,
  });
  final int port;
  final bool isOpen;
  final Duration? responseTime;

  @override
  String toString() => 'OpenPort($port, open: $isOpen)';
}

/// Progress of an ongoing scan.
class ScanProgress {

  const ScanProgress({
    required this.scanned,
    required this.total,
    this.found = const [],
    this.lastIp,
  });
  final int scanned;
  final int total;
  final List<ScannedHost> found;

  /// The last IP address that was checked — gives the UI something to show
  /// between progress jumps so the scan always looks active.
  final String? lastIp;

  double get percent => total > 0 ? scanned / total : 0;
}
