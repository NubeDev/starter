/// Models for network interface selection.
library;

import 'package:flutter/foundation.dart';

/// The guessed type of a network interface based on its name.
enum InterfaceType {
  ethernet,
  wifi,
  loopback,
  vpn,
  bridge,
  other;

  static InterfaceType fromName(String name) {
    final n = name.toLowerCase();
    if (n == 'lo' || n.startsWith('loopback')) return InterfaceType.loopback;
    if (n.startsWith('eth') ||
        n.startsWith('en') ||
        n.startsWith('eno') ||
        n.startsWith('enp')) {
      return InterfaceType.ethernet;
    }
    if (n.startsWith('wlan') ||
        n.startsWith('wlp') ||
        n.startsWith('wl') ||
        n.startsWith('wifi')) {
      return InterfaceType.wifi;
    }
    if (n.startsWith('tun') || n.startsWith('tap') || n.startsWith('vpn')) {
      return InterfaceType.vpn;
    }
    if (n.startsWith('br') ||
        n.startsWith('virbr') ||
        n.startsWith('docker') ||
        n.startsWith('veth')) {
      return InterfaceType.bridge;
    }
    return InterfaceType.other;
  }

  String get label {
    switch (this) {
      case InterfaceType.ethernet:
        return 'Ethernet';
      case InterfaceType.wifi:
        return 'Wi-Fi';
      case InterfaceType.loopback:
        return 'Loopback';
      case InterfaceType.vpn:
        return 'VPN';
      case InterfaceType.bridge:
        return 'Bridge';
      case InterfaceType.other:
        return 'Other';
    }
  }
}

/// A network interface with its associated addresses.
@immutable
class NetworkInterfaceInfo {
  const NetworkInterfaceInfo({
    required this.name,
    required this.addresses,
    required this.type,
  });

  final String name;

  /// All IP addresses bound to this interface (IPv4 and IPv6).
  final List<String> addresses;

  final InterfaceType type;

  /// The first IPv4 address, if any.
  String? get primaryAddress =>
      addresses.where((a) => !a.contains(':')).firstOrNull;

  /// Display-friendly label, e.g. "eth0 (192.168.1.5)"
  String get displayLabel {
    final addr = primaryAddress;
    return addr != null ? '$name  ($addr)' : name;
  }

  @override
  String toString() => 'NetworkInterfaceInfo($name, $addresses, $type)';

  @override
  bool operator ==(Object other) =>
      other is NetworkInterfaceInfo && other.name == name;

  @override
  int get hashCode => name.hashCode;
}
