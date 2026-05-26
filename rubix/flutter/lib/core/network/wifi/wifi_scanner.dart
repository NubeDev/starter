import 'dart:async';
import 'dart:io';

import 'package:wifi_scan/wifi_scan.dart';

import 'wifi_models.dart';

export 'wifi_models.dart';

/// Cross-platform WiFi network scanner.
///
/// Uses [wifi_scan] package which supports Android, iOS, Windows, and Linux.
///
/// Usage:
/// ```dart
/// final scanner = WifiScanner();
/// final canScan = await scanner.canScan();
/// if (canScan) {
///   final result = await scanner.scan();
///   for (final network in result.sortedBySignal) {
///     print('${network.displayName}: ${network.rssi}dBm');
///   }
/// }
/// ```
class WifiScanner {
  final WiFiScan _wifiScan;

  WifiScanner() : _wifiScan = WiFiScan.instance;

  /// Whether the current platform supports WiFi scanning.
  bool get isPlatformSupported =>
      Platform.isAndroid ||
      Platform.isIOS ||
      Platform.isWindows ||
      Platform.isLinux;

  /// Checks if scanning is possible (permissions granted, WiFi enabled, etc).
  Future<bool> canScan() async {
    if (!isPlatformSupported) return false;
    try {
      final can = await _wifiScan.canStartScan(askPermissions: true);
      return can == CanStartScan.yes;
    } catch (_) {
      return false;
    }
  }

  /// Checks if we can retrieve previous scan results.
  Future<bool> canGetResults() async {
    if (!isPlatformSupported) return false;
    try {
      final can = await _wifiScan.canGetScannedResults(askPermissions: true);
      return can == CanGetScannedResults.yes;
    } catch (_) {
      return false;
    }
  }

  /// Triggers a WiFi scan and returns the results.
  Future<WifiScanResult> scan() async {
    if (!isPlatformSupported) {
      return WifiScanResult(
        scannedAt: DateTime.now(),
        error: 'WiFi scanning not supported on ${Platform.operatingSystem}',
      );
    }

    try {
      final canStart = await _wifiScan.canStartScan(askPermissions: true);
      if (canStart == CanStartScan.yes) {
        await _wifiScan.startScan();
        await Future.delayed(const Duration(seconds: 2));
      }

      return await getLastResults();
    } catch (e) {
      return WifiScanResult(
        scannedAt: DateTime.now(),
        error: 'Scan failed: $e',
      );
    }
  }

  /// Returns the most recent scan results without triggering a new scan.
  Future<WifiScanResult> getLastResults() async {
    if (!isPlatformSupported) {
      return WifiScanResult(
        scannedAt: DateTime.now(),
        error: 'WiFi scanning not supported on ${Platform.operatingSystem}',
      );
    }

    try {
      final canGet =
          await _wifiScan.canGetScannedResults(askPermissions: true);
      if (canGet != CanGetScannedResults.yes) {
        return WifiScanResult(
          scannedAt: DateTime.now(),
          error: 'Cannot get scan results: $canGet',
        );
      }

      final accessPoints = await _wifiScan.getScannedResults();
      final networks = accessPoints.map(_toWifiNetwork).toList();

      return WifiScanResult(
        networks: networks,
        scannedAt: DateTime.now(),
      );
    } catch (e) {
      return WifiScanResult(
        scannedAt: DateTime.now(),
        error: 'Failed to get results: $e',
      );
    }
  }

  /// Streams scan results at the given interval.
  Stream<WifiScanResult> periodicScan({
    Duration interval = const Duration(seconds: 10),
  }) {
    late final StreamController<WifiScanResult> controller;
    Timer? timer;

    controller = StreamController<WifiScanResult>(
      onListen: () {
        scan().then(controller.add);
        timer = Timer.periodic(interval, (_) {
          scan().then(controller.add);
        });
      },
      onCancel: () {
        timer?.cancel();
      },
    );

    return controller.stream;
  }

  /// Subscribes to the platform's scan result stream.
  Stream<WifiScanResult> get onResultsAvailable {
    return _wifiScan.onScannedResultsAvailable.map((accessPoints) {
      final networks = accessPoints.map(_toWifiNetwork).toList();
      return WifiScanResult(
        networks: networks,
        scannedAt: DateTime.now(),
      );
    });
  }

  static int? _channelWidthToMhz(WiFiChannelWidth? width) {
    if (width == null) return null;
    switch (width) {
      case WiFiChannelWidth.mhz20:
        return 20;
      case WiFiChannelWidth.mhz40:
        return 40;
      case WiFiChannelWidth.mhz80:
        return 80;
      case WiFiChannelWidth.mhz160:
        return 160;
      case WiFiChannelWidth.mhz80Plus80:
        return 160;
      case WiFiChannelWidth.unkown:
        return null;
    }
  }

  WifiNetwork _toWifiNetwork(WiFiAccessPoint ap) {
    return WifiNetwork(
      ssid: ap.ssid,
      bssid: ap.bssid,
      rssi: ap.level,
      frequencyMhz: ap.frequency,
      capabilities: ap.capabilities,
      channelWidthMhz: _channelWidthToMhz(ap.channelWidth),
    );
  }
}
