import 'dart:async';
import 'dart:io';

import 'package:rubix_flutter/core/network/scanner_models.dart';

/// Checks specific TCP ports on a given host.
class PortScanner {

  const PortScanner({
    this.timeout = const Duration(milliseconds: 1500),
  });
  /// Timeout for each port connection attempt.
  final Duration timeout;

  /// Checks whether a single TCP port is open on [host].
  Future<OpenPort> checkPort(String host, int port) async {
    final sw = Stopwatch()..start();
    try {
      final socket = await Socket.connect(host, port, timeout: timeout);
      sw.stop();
      socket.destroy();
      return OpenPort(port: port, isOpen: true, responseTime: sw.elapsed);
    } catch (_) {
      sw.stop();
      return OpenPort(port: port, isOpen: false);
    }
  }

  /// Checks multiple ports on [host] concurrently.
  /// Returns only the open ports.
  Future<List<OpenPort>> scanPorts(String host, List<int> ports) async {
    final results = await Future.wait(
      ports.map((p) => checkPort(host, p)),
    );
    return results.where((r) => r.isOpen).toList();
  }

  /// Scans a range of ports on [host].
  Future<List<OpenPort>> scanRange(
    String host,
    int startPort,
    int endPort,
  ) async {
    final ports =
        List.generate(endPort - startPort + 1, (i) => startPort + i);
    return scanPorts(host, ports);
  }
}
