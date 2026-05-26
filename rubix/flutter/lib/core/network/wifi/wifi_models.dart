/// Security type of a WiFi network.
enum WifiSecurity {
  open,
  wep,
  wpa,
  wpa2,
  wpa3,
  unknown;

  static WifiSecurity fromCapabilities(String capabilities) {
    final caps = capabilities.toUpperCase();
    if (caps.contains('WPA3')) return WifiSecurity.wpa3;
    if (caps.contains('WPA2')) return WifiSecurity.wpa2;
    if (caps.contains('WPA')) return WifiSecurity.wpa;
    if (caps.contains('WEP')) return WifiSecurity.wep;
    if (caps.contains('ESS') &&
        !caps.contains('WPA') &&
        !caps.contains('WEP')) {
      return WifiSecurity.open;
    }
    return WifiSecurity.unknown;
  }
}

/// Frequency band of a WiFi network.
enum WifiBand {
  band2_4GHz,
  band5GHz,
  band6GHz,
  unknown;

  static WifiBand fromFrequency(int? frequencyMhz) {
    if (frequencyMhz == null) return WifiBand.unknown;
    if (frequencyMhz >= 2400 && frequencyMhz <= 2500) {
      return WifiBand.band2_4GHz;
    }
    if (frequencyMhz >= 5150 && frequencyMhz <= 5900) {
      return WifiBand.band5GHz;
    }
    if (frequencyMhz >= 5925 && frequencyMhz <= 7125) {
      return WifiBand.band6GHz;
    }
    return WifiBand.unknown;
  }

  String get label {
    switch (this) {
      case WifiBand.band2_4GHz:
        return '2.4 GHz';
      case WifiBand.band5GHz:
        return '5 GHz';
      case WifiBand.band6GHz:
        return '6 GHz';
      case WifiBand.unknown:
        return 'Unknown';
    }
  }
}

/// Signal strength category derived from RSSI.
enum SignalStrength {
  excellent, // >= -50 dBm
  good, // -50 to -60
  fair, // -60 to -70
  weak, // -70 to -80
  veryWeak; // < -80

  static SignalStrength fromRssi(int? rssi) {
    if (rssi == null) return SignalStrength.veryWeak;
    if (rssi >= -50) return SignalStrength.excellent;
    if (rssi >= -60) return SignalStrength.good;
    if (rssi >= -70) return SignalStrength.fair;
    if (rssi >= -80) return SignalStrength.weak;
    return SignalStrength.veryWeak;
  }

  /// Returns a 0-100 percentage for UI display.
  int get percentage {
    switch (this) {
      case SignalStrength.excellent:
        return 100;
      case SignalStrength.good:
        return 75;
      case SignalStrength.fair:
        return 50;
      case SignalStrength.weak:
        return 25;
      case SignalStrength.veryWeak:
        return 10;
    }
  }
}

/// A discovered WiFi network (access point).
class WifiNetwork {

  WifiNetwork({
    required this.ssid,
    required this.bssid,
    this.rssi,
    this.frequencyMhz,
    this.capabilities = '',
    this.channelWidthMhz,
    DateTime? scannedAt,
  }) : scannedAt = scannedAt ?? DateTime.now();
  /// Network name (SSID). Empty string means hidden network.
  final String ssid;

  /// MAC address of the access point.
  final String bssid;

  /// Signal strength in dBm (typically -30 to -100).
  final int? rssi;

  /// Frequency in MHz.
  final int? frequencyMhz;

  /// Raw capabilities string from the scan.
  final String capabilities;

  /// Channel width in MHz if available.
  final int? channelWidthMhz;

  /// Timestamp of when the network was seen.
  final DateTime scannedAt;

  /// Whether this is a hidden network.
  bool get isHidden => ssid.isEmpty;

  /// Display name, showing `<Hidden>` for unnamed networks.
  String get displayName => isHidden ? '<Hidden>' : ssid;

  /// Derived security type.
  WifiSecurity get security => WifiSecurity.fromCapabilities(capabilities);

  /// Derived frequency band.
  WifiBand get band => WifiBand.fromFrequency(frequencyMhz);

  /// Derived signal strength category.
  SignalStrength get signalStrength => SignalStrength.fromRssi(rssi);

  /// WiFi channel number derived from frequency.
  int? get channel {
    if (frequencyMhz == null) return null;
    // 2.4 GHz band
    if (frequencyMhz! >= 2412 && frequencyMhz! <= 2484) {
      if (frequencyMhz == 2484) return 14;
      return ((frequencyMhz! - 2412) ~/ 5) + 1;
    }
    // 5 GHz band
    if (frequencyMhz! >= 5170 && frequencyMhz! <= 5825) {
      return ((frequencyMhz! - 5170) ~/ 5) + 34;
    }
    return null;
  }

  @override
  String toString() =>
      'WifiNetwork($displayName, ${rssi}dBm, ${band.label}, '
      '${security.name})';
}

/// Result of a WiFi scan operation.
class WifiScanResult {

  const WifiScanResult({
    required this.scannedAt, this.networks = const [],
    this.error,
  });
  final List<WifiNetwork> networks;
  final DateTime scannedAt;
  final String? error;

  bool get hasError => error != null;
  bool get isEmpty => networks.isEmpty && !hasError;
  int get count => networks.length;

  /// Networks sorted by signal strength (strongest first).
  List<WifiNetwork> get sortedBySignal {
    final sorted = List<WifiNetwork>.from(networks)
      ..sort((a, b) => (b.rssi ?? -100).compareTo(a.rssi ?? -100));
    return sorted;
  }

  /// Unique networks by SSID (keeps the strongest signal per SSID).
  List<WifiNetwork> get uniqueBySSID {
    final map = <String, WifiNetwork>{};
    for (final n in sortedBySignal) {
      final key = n.ssid.isEmpty ? n.bssid : n.ssid;
      map.putIfAbsent(key, () => n);
    }
    return map.values.toList();
  }
}
